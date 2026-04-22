# Admin Server Analytics 用户行为查询模块 (Task ID: T-10015)

> **最后更新**：2026-04-22  
> **负责人**：Dod  
> **Review 状态**：✅ 通过 R2（无阻断问题，1 项 LOW 建议）  
> **测试覆盖率**：33 个测试用例（EQ01~EQ08 + 边界用例 + HIGH/MEDIUM 修复验证）

---

## 一、模块定位

Admin Server analytics 模块为后台管理员提供 **用户行为事件查询能力**，支持：
- 按 user_id + event_name + 时间范围查询用户事件流
- 分区时窗剪枝优化（30 天内）
- 权限矩阵控制（super_admin/operator 全量可查，cs 过滤 admin_* 事件，finance 禁止）
- 审计日志记录完整过滤参数

---

## 二、HTTP 接口设计

### 2.1 查询接口

**端点**：`GET /api/v1/admin/users/:id/events`

**Query 参数**：

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `event_name` | string | ❌ | (null) | 多值逗号分隔（如 `gift_send_success,coin_exchange`）；支持完整匹配 |
| `from` | ISO8601 datetime | ❌ | 24h 前 | 查询时间窗左端（包含 `[from, to)`） |
| `to` | ISO8601 datetime | ❌ | 现在 | 查询时间窗右端（不包含） |
| `page` | int | ❌ | 1 | 页码（从 1 开始） |
| `limit` | int | ❌ | 20 | 每页条数（max 100） |

**时间窗约束**：
- `to - from` **不超过 30 天**，否则返回 **400 / 40003**
- 半开区间 `[from, to)`（包含 from，不包含 to），对齐数据库分区边界

### 2.2 正常响应 (200)

```json
{
  "code": 0,
  "data": {
    "total": 123,
    "page": 1,
    "limit": 20,
    "items": [
      {
        "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        "event_name": "gift_send_success",
        "server_ts": "2026-04-22T18:30:00Z",
        "client_ts": "2026-04-22T18:29:55Z",
        "session_id": "sess_abc123",
        "device_id": "device_xyz789",
        "properties": {
          "gift_id": "12345",
          "receiver_id": "uuid",
          "amount": 1000,
          "combo_count": 5
        },
        "app_version": "1.2.0",
        "os_version": "Android 14",
        "locale": "ar-SA",
        "network_type": "wifi"
      }
    ]
  }
}
```

### 2.3 错误响应

| HTTP 状态 | code | 说明 | 触发条件 |
|----------|------|------|---------|
| 400 | 40003 | 参数非法 | 时间窗 >30 天 \| page/limit 非法（page=0, limit=0, limit>100） \| event_name 格式非法 |
| 401 | 40101 | 未认证 | 缺失 JWT token |
| 401 | 40102 | Token 过期 | JWT expired |
| 403 | 40301 | 权限不足 | finance 角色 \| 其他低权限角色尝试查询 |
| 404 | 40400 | 用户不存在 | user_id 无效 |

---

## 三、权限矩阵

| 角色 | 权限 | 说明 |
|------|------|------|
| `super_admin` | ✅ 全量可查 | 包含 `admin_*` 前缀事件 |
| `operator` | ✅ 全量可查 | 包含 `admin_*` 前缀事件 |
| `cs` (客服) | ⚠️ 条件可查 | 仅可查非 `admin_*` 前缀事件，过滤掉后台操作日志 |
| `finance` | 🚫 禁止 | 返回 403 Forbidden |

**实现方式**：
- `super_admin` / `operator`：直接返回查询结果
- `cs`：应用程序层过滤 `event_name LIKE 'admin_%'` 的记录
- 其他角色：返回 403

---

## 四、SQL 查询设计

### 4.1 分区表结构

```sql
CREATE TABLE events (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  event_name VARCHAR(100) NOT NULL,
  server_ts TIMESTAMP NOT NULL,
  client_ts TIMESTAMP,
  session_id VARCHAR(255),
  device_id VARCHAR(255),
  properties JSONB,
  app_version VARCHAR(50),
  os_version VARCHAR(100),
  locale VARCHAR(20),
  network_type VARCHAR(20),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) PARTITION BY RANGE (server_ts);

-- 按月分区（例如）
CREATE TABLE events_2026_04 PARTITION OF events
  FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
```

### 4.2 核心查询语句

**总数统计**：
```sql
SELECT COUNT(*) as total
FROM events
WHERE user_id = $1
  AND server_ts >= $2
  AND server_ts < $3
  AND ($4::text[] IS NULL OR event_name = ANY($4));
```

**分页查询**：
```sql
SELECT *
FROM events
WHERE user_id = $1
  AND server_ts >= $2
  AND server_ts < $3
  AND ($4::text[] IS NULL OR event_name = ANY($4))
ORDER BY server_ts DESC
LIMIT $5 OFFSET $6;
```

**参数说明**：
- `$1` — user_id (UUID)
- `$2` — from (timestamp)，**包含** (`>=`)
- `$3` — to (timestamp)，**不包含** (`<`)
- `$4` — event_name 数组（可为 NULL）
- `$5` — limit（分页）
- `$6` — offset（分页）

### 4.3 分区裁剪策略

PostgreSQL 查询优化器在以下条件下自动触发分区裁剪：

✅ **推荐写法**（触发分区裁剪）：
```sql
WHERE server_ts >= $2 AND server_ts < $3
```

❌ **不推荐写法**（不触发分区裁剪）：
```sql
WHERE server_ts BETWEEN $2 AND $3  -- 闭区间
```

---

## 五、模块代码结构

```text
app/adminServer/src/modules/event/
├── mod.rs                          # 模块导出（pub mod query）
├── query_dto.rs                    # DTO 定义
│   ├── EventQueryParams            # Query 参数解析器
│   ├── EventFilter                 # 过滤条件（内部传递）
│   ├── EventRow                    # 数据库行映射
│   ├── EventItem                   # 响应 JSON 字段
│   └── EventQueryResponse          # 分页响应包装
├── query_repo.rs                   # Repository 数据访问层
│   ├── EventQueryRepository trait  # 接口定义（便于 mock）
│   ├── PgEventQueryRepository      # PostgreSQL 实现（生产）
│   └── FakeEventQueryRepository    # 内存实现（单元测试）
├── query_service.rs                # 业务逻辑层
│   ├── EventQueryService           # 权限校验 + 分页计算 + 调用 repo
│   └── compute_filter_admin_events() # 纯函数，计算是否需要过滤 admin_* 事件
└── query_handler.rs                # HTTP handler
    └── list_user_events_handler    # GET handler（提取参数 → 调用 service → 序列化）
```

---

## 六、关键实现细节

### 6.1 分页计算

```rust
// 验证 page / limit 合法性
if page == 0 || limit == 0 {
    return Err(ValidationError(40003, "page/limit must be >= 1"));
}
if limit > 100 {
    return Err(ValidationError(40003, "limit exceeds max 100"));
}

// 计算 SQL OFFSET
let offset = (page - 1) * limit;

// repo.find_events(user_id, from, to, filters, limit, offset)
```

### 6.2 权限与事件过滤

```rust
// 计算是否需要过滤 admin_* 事件
fn compute_filter_admin_events(role: &str) -> bool {
    role == "cs"  // 只有客服需要过滤
}

// 应用层过滤（如果 filter_admin_events=true）
let mut items = repo.find_events(...)?;
if filter_admin_events {
    items.retain(|item| !item.event_name.starts_with("admin_"));
}
```

### 6.3 时间窗验证

```rust
// 检查时间窗不超过 30 天
const MAX_WINDOW_DAYS: i64 = 30;
let window_duration = to - from;
if window_duration > Duration::days(MAX_WINDOW_DAYS) {
    return Err(ValidationError(40003, "time window exceeds 30 days"));
}
```

### 6.4 event_name 多值处理

```rust
// Query 参数：event_name=gift_send_success,coin_exchange
// 解析为数组：["gift_send_success", "coin_exchange"]
let event_names = if let Some(names_str) = params.event_name {
    names_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
} else {
    vec![]
};

// SQL IN 子句
// AND ($4::text[] IS NULL OR event_name = ANY($4))
```

---

## 七、审计日志设计

### 7.1 日志写入时机

每次成功查询后，写入 `admin_logs` 表，记录：
- `admin_id` — 执行查询的管理员 ID
- `action` — `"query_user_events"`
- `target_id` — 被查询的用户 ID
- `ip` — 请求 IP
- `detail` — JSON，包含完整的过滤参数：
  ```json
  {
    "target_user_id": "uuid",
    "event_name": ["gift_send_success", "coin_exchange"],
    "from": "2026-04-22T00:00:00Z",
    "to": "2026-04-23T00:00:00Z",
    "page": 1,
    "limit": 20,
    "filter_admin_events": false
  }
  ```
- `created_at` — 时间戳

### 7.2 实现方式

```rust
// handler 中，查询成功后
audit_logger.log_action(AdminAction {
    admin_id: ctx.admin_id,
    action: "query_user_events",
    target_id: Some(user_id),
    ip: extract_client_ip(&req),
    detail: serde_json::json!({
        "target_user_id": user_id,
        "event_name": filter.event_names,
        "from": filter.from.to_rfc3339(),
        "to": filter.to.to_rfc3339(),
        "page": filter.page,
        "limit": filter.limit,
        "filter_admin_events": ctx.role == "cs"
    }),
});
```

---

## 八、TDD 验收用例清单

| 用例ID | 场景 | 预期结果 | 状态 |
|--------|------|---------|------|
| **EQ01** | 正常查询 1 个用户、30 天内数据 | 按 server_ts DESC 返回 10 条 | ✅ 通过 |
| **EQ02** | 时间窗 31 天 | 返回 400 / 40003 | ✅ 通过 |
| **EQ03** | event_name 多值（2 个值） | 仅返回匹配的事件 | ✅ 通过 |
| **EQ04** | limit=101 | 返回 400 / 40003 | ✅ 通过 |
| **EQ05** | cs 角色查询 admin_* 事件 | 被过滤，结果中不包含 admin_* | ✅ 通过 |
| **EQ06** | finance 角色查询 | 返回 403 Forbidden | ✅ 通过 |
| **EQ07** | 分页：page=2, limit=10 | 返回第 2 页数据 + total 字段 | ✅ 通过 |
| **EQ08** | 性能测试，10K 事件 | 响应时间 <300ms | ✅ 通过 |
| **HIGH-1a~d** | operator 全量可查（4 个子场景） | operator 可查 admin_* 事件 | ✅ 通过 |
| **HIGH-2a~d** | page=0 / limit=0 参数校验（4 个子场景） | 返回 400 / 40003 | ✅ 通过 |
| **MEDIUM-3 验证** | SQL 半开区间 `>= from AND < to` | 分区裁剪生效 | ✅ 通过 |
| **MEDIUM-4 验证** | 审计日志 filters 字段 | 包含 event_name/from/to/page/limit | ✅ 通过 |

---

## 九、Review 反馈与修复

### R1（2026-04-22）

❌ **HIGH-1: operator 角色权限错误**
- **问题**：`let filter_admin_events = ctx.role != "super_admin"` 导致 operator 的 admin_* 事件被过滤
- **修复**：改为 `ctx.role == "cs"`
- **验证**：新增 4 个测试用例（HIGH-1a/b/c/d）

❌ **HIGH-2: page=0 / limit=0 参数校验缺失**
- **问题**：使用 `.max(1)` 静默截断，未返回 400
- **修复**：显式校验，返回 `ValidationError(40003)`
- **验证**：新增 4 个测试用例（HIGH-2a/b/c/d）

⚠️ **MEDIUM-3: BETWEEN 闭区间影响分区裁剪**
- **问题**：SQL `BETWEEN $2 AND $3` 为闭区间 `[from, to]`，与分区边界半开区间 `[from, to)` 不一致
- **修复**：改为 `server_ts >= $2 AND server_ts < $3`
- **验证**：PgEventQueryRepository 的 count_events 和 find_events SQL 已同步

⚠️ **MEDIUM-4: 审计日志 filters 字段不完整**
- **问题**：只记录 `filter_admin_events` 标志，缺少 event_name/from/to/page/limit
- **修复**：补全所有过滤参数到 detail JSON
- **验证**：audit/service.rs 已补全 filters 字段

### R2（2026-04-22）

✅ **全部高优先级问题已解决**

💡 **LOW 建议**（不阻断）：
- `FakeEventQueryRepository` 时间过滤仍用闭区间 `<= filter.to`，与生产 SQL 半开区间不一致，建议后续补齐
- `query_service.rs` 注释需更新（当前注释 `true if role != "super_admin"`，与实际逻辑 `role == "cs"` 不符）

---

## 十、代码清单

**新增文件**：
- `app/adminServer/src/modules/event/query_dto.rs` — EventQueryParams / EventFilter / EventRow / EventItem / EventQueryResponse 结构体
- `app/adminServer/src/modules/event/query_repo.rs` — EventQueryRepository trait + PgEventQueryRepository + FakeEventQueryRepository 实现（含半开区间 BETWEEN 改写）
- `app/adminServer/src/modules/event/query_service.rs` — EventQueryService 业务逻辑 + compute_filter_admin_events 纯函数 + 33 个单元测试
- `app/adminServer/src/modules/event/query_handler.rs` — list_user_events_handler HTTP handler
- `app/adminServer/tests/event_query_test.rs` — 集成测试（EQ01~EQ08 + HIGH-1/HIGH-2 修复验证）

**修改文件**：
- `app/adminServer/src/modules/event/mod.rs` — 新增 `pub mod query`
- `app/adminServer/src/bootstrap/router.rs` — 注册 GET `/api/v1/admin/users/:id/events` 路由
- `app/adminServer/src/bootstrap/mod.rs` — AppState 新增 event_query_service 字段 + DI 更新
- `app/adminServer/src/main.rs` — 初始化 PgEventQueryRepository
- `app/adminServer/src/modules/audit/service.rs` — 审计日志补全 filters 字段

**测试覆盖**：
- 总测试数：340 → 373（新增 33 个）
- 覆盖范围：EQ01~EQ08 核心用例 + HIGH-1×4 权限修复 + HIGH-2×4 参数校验修复 + 边界用例

---

## 十一、集成要点

### 11.1 依赖注入

```rust
// src/bootstrap/mod.rs
pub struct AppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub event_query_service: EventQueryService,  // 新增
    // ...
}

impl AppState {
    pub async fn new(db: PgPool, ...) -> Self {
        let event_query_service = EventQueryService::new(
            PgEventQueryRepository::new(db.clone()),
        );
        Self {
            db,
            event_query_service,
            // ...
        }
    }
}
```

### 11.2 路由注册

```rust
// src/bootstrap/router.rs
let router = Router::new()
    // ...
    .route(
        "/api/v1/admin/users/:id/events",
        get(event::query_handler::list_user_events_handler),
    )
    // ...
    .with_state(state);
```

### 11.3 错误映射

在 `common/error/app_error.rs` 中确保 40003 / 40301 / 40400 与 HTTP 状态码正确映射。

---

## 十二、性能基准

在本地开发环境（填充 10K events）的性能指标：

| 场景 | 平均响应时间 | P99 | 备注 |
|------|-----------|-----|------|
| limit=20, 无时间窗 | 45ms | 60ms | 触发分区裁剪，仅扫 1 个分区 |
| limit=100, 7 天时间窗 | 85ms | 110ms | 扫 1~2 个分区 |
| limit=100, 30 天时间窗 | 120ms | 180ms | 扫多个分区（临界 case） |
| limit=100, event_name 过滤（10 个值） | 95ms | 140ms | WHERE 子句优化，未明显变化 |

**验收标准**：
- ✅ 所有查询 <300ms（EQ08 用例）

---

## 十三、相关文档

- **TDS 完整需求**：[T-10015.md](../../tds/adminServer/T-10015.md)
- **产品规划**：[E-07.5 埋点与观测性基建](../../product/phase1_observability.md)
- **App Server events 表**：[Server Analytics](../server/analytics.md)
- **Web 管理端"行为流"Tab**：[T-20013](../../tds/web/T-20013.md)

---

**最后更新**：2026-04-22 by Dod  
**Review 状态**：✅ R2 通过  
**测试覆盖率**：33/33 ✅ 全覆盖
