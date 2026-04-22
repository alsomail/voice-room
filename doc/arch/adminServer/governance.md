<!--
[AI 读写指令与维护规约]
1. 本文件记录 Admin Server 治理日志查询模块（T-10016）架构与接口设计。
2. 治理日志分为踢人记录（kicks）和禁言记录（mutes）两个独立接口。
3. 所有接口均需 Admin JWT，GovernanceRead 权限控制。
4. 与 audit 模块联动：每次查询均写入 admin_logs 审计记录。
-->

# Admin Server 治理日志查询模块

**最后更新：** 2026-05-20
**Task ID：** T-10016（Review Round 2 通过 ✅）
**入口点：** `app/adminServer/src/modules/governance/`

---

## 一、架构概述

治理日志查询模块为运营/客服人员提供房间踢人、禁言操作的审计日志查询能力，支持多维度过滤与分页。

```
GET /api/v1/admin/governance/kicks   ← 踢人日志查询
GET /api/v1/admin/governance/mutes   ← 禁言日志查询
         │
         ▼
  GovernanceHandler (handler.rs)
         │  RBAC: Permission::GovernanceRead
         ▼
  GovernanceService (service.rs)
    ├── resolve_params() — 参数解析 + 校验
    │     ├── 时间范围 ≤ 90 天
    │     ├── page ≥ 1
    │     ├── limit clamp(1, 100)
    │     └── mute_type ∈ {"mic", "chat"}
    ├── AuditLogger.log_action()  — fire-and-forget 审计写入
    └── GovernanceRepo (repo.rs)
          ├── PgGovernanceRepo   — 生产环境（SQLx + PostgreSQL）
          └── FakeGovernanceRepo — 单元/集成测试桩
```

---

## 二、API 接口

### 2.1 GET /api/v1/admin/governance/kicks — 踢人日志

**权限**：`GovernanceRead`（super_admin / operator / cs 可访问；finance → 403）

#### 查询参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `room_id` | UUID | 否 | 按房间过滤 |
| `target_user_id` | UUID | 否 | 按被踢用户过滤 |
| `operator_user_id` | UUID | 否 | 按操作管理员过滤 |
| `from` | RFC3339 | 否 | 时间范围起始，默认 7 天前 |
| `to` | RFC3339 | 否 | 时间范围结束，默认当前时间 |
| `page` | i64 | 否 | 页码（1-based），默认 1 |
| `limit` | i64 | 否 | 每页条数，默认 20，最大 100 |

#### 响应格式

```json
{
  "code": 0,
  "request_id": "uuid",
  "data": {
    "total": 12,
    "page": 1,
    "limit": 20,
    "items": [
      {
        "id": "uuid",
        "room_id": "uuid",
        "room_title": "阿拉伯语聊天室",
        "target_user_id": "uuid",
        "target_nickname": "用户昵称",
        "operator_user_id": "uuid",
        "operator_nickname": "管理员昵称",
        "reason": "harassment",
        "created_at": "2026-05-20T10:00:00Z"
      }
    ]
  }
}
```

### 2.2 GET /api/v1/admin/governance/mutes — 禁言日志

**权限**：同上（`GovernanceRead`）

#### 查询参数

在 kicks 参数基础上新增：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | 否 | 禁言类型：`mic`（禁麦）或 `chat`（禁聊）；不传则不过滤 |

> ⚠️ 参数名为 `type`（URL 查询参数），服务层内部字段名为 `mute_type`。

#### 响应格式

比 kicks 多 `mute_type` 与 `duration_sec` 字段：

```json
{
  "code": 0,
  "request_id": "uuid",
  "data": {
    "total": 8,
    "page": 1,
    "limit": 20,
    "items": [
      {
        "id": "uuid",
        "room_id": "uuid",
        "room_title": "阿拉伯语聊天室",
        "target_user_id": "uuid",
        "target_nickname": "用户昵称",
        "operator_user_id": "uuid",
        "operator_nickname": "管理员昵称",
        "mute_type": "mic",
        "duration_sec": 300,
        "reason": "spam",
        "created_at": "2026-05-20T10:00:00Z"
      }
    ]
  }
}
```

> `duration_sec = null` 表示永久禁言。

---

## 三、权限矩阵

| 角色 | /governance/kicks | /governance/mutes |
|------|:-----------------:|:-----------------:|
| super_admin | ✅ | ✅ |
| operator | ✅ | ✅ |
| cs（客服） | ✅ | ✅ |
| finance | ❌ 403 | ❌ 403 |

权限标识：`Permission::GovernanceRead`（定义于 `common/auth/context.rs`）

---

## 四、校验规则

| 规则 | 说明 | 错误码 |
|------|------|--------|
| `page < 1`（含负值与 0）| 页码非正整数 | 40003 ValidationError |
| `to - from > 90 天` | 时间窗超限 | 40003 ValidationError |
| `mute_type ∉ {"mic", "chat"}` | 枚举非法值（大小写敏感，空字符串也非法）| 40003 ValidationError |
| `limit > 100` | 截断为 100，**不**报错 | — |
| `from`/`to` 格式非法 | 无法解析 RFC3339 | 40003 ValidationError |

---

## 五、数据层

### 5.1 依赖数据表

| 表名 | 说明 |
|------|------|
| `room_kick_records` | 踢人记录（T-00028 创建） |
| `room_mute_records` | 禁言记录（T-00029 创建） |
| `rooms` | JOIN 获取 `room_title` |
| `users` | JOIN 两次获取 `target_nickname` / `operator_nickname` |

### 5.2 核心 SQL（kicks）

```sql
SELECT k.*, r.title AS room_title,
       tu.nickname AS target_nickname,
       ou.nickname AS operator_nickname
  FROM room_kick_records k
  JOIN rooms r  ON r.id = k.room_id
  JOIN users tu ON tu.id = k.target_user_id
  JOIN users ou ON ou.id = k.operator_user_id
 WHERE ($1::uuid IS NULL OR k.room_id         = $1)
   AND ($2::uuid IS NULL OR k.target_user_id  = $2)
   AND ($3::uuid IS NULL OR k.operator_user_id = $3)
   AND k.created_at BETWEEN $4 AND $5
 ORDER BY k.created_at DESC
 LIMIT $6 OFFSET $7;
```

mutes 查询额外追加 `AND ($8::text IS NULL OR k.mute_type = $8)`。

---

## 六、审计日志

每次查询成功后，以 fire-and-forget 方式写入 `admin_logs`：

| 字段 | kicks 值 | mutes 值 |
|------|---------|---------|
| `action` | `query_kick_records` | `query_mute_records` |
| `target_id` | `"governance"` | `"governance"` |
| `detail` | 包含完整 filters（room_id/target_user_id/operator_user_id/from/to/page/limit） | 同 kicks + mute_type |

---

## 七、模块文件清单

```text
app/adminServer/src/modules/governance/
├── mod.rs         — 模块声明，re-export GovernanceService
├── handler.rs     — list_kicks_handler / list_mutes_handler（Axum Handler）
├── service.rs     — GovernanceService（业务逻辑 + 参数校验 + 审计）
└── repo.rs        — GovernanceRepo trait / PgGovernanceRepo / FakeGovernanceRepo

app/adminServer/tests/
└── governance_logs_test.rs  — G16-01~G16-08 集成测试（共 24 个用例）
```

路由注册位于 `app/adminServer/src/bootstrap/mod.rs`：

```
GET /api/v1/admin/governance/kicks  → list_kicks_handler
GET /api/v1/admin/governance/mutes  → list_mutes_handler
```

---

## 八、测试覆盖汇总

| 测试层 | 用例范围 | 数量 | 状态 |
|--------|---------|------|------|
| 单元测试（service.rs inline） | SV-01~SV-16（含 Review R1 新增 SV-09~SV-16） | 16 | ✅ 全通过 |
| Handler 测试（handler.rs inline） | 权限矩阵 / 401 场景 | 5 | ✅ 全通过 |
| 集成测试（governance_logs_test.rs） | G16-01~G16-08 + 边界用例 | 24 | ✅ 全通过 |
| **合计（含其他模块）** | — | **419** | ✅ **全通过** |

### 关键验收用例

| 用例 | 场景 | 期望结果 |
|------|------|---------|
| G16-01 | 查询结果排序 | 按 created_at DESC |
| G16-02 | 时间窗 > 90 天 | → 40003 |
| G16-03 | room_id 过滤 | 仅返回指定房间记录 |
| G16-04 | target_user_id 过滤 | 仅返回指定用户记录 |
| G16-05 | mutes type 过滤 | type=mic 不返回 chat 记录 |
| G16-06 | finance 角色 | → 403 |
| G16-07 | 查询写入 admin_logs | action = query_kick/mute_records |
| G16-08 | page=0 / page=-1 | → 40003 |

---

## 九、Review 修复记录

### Round 1（未通过）→ Round 2（通过 ✅）

| 级别 | 问题 | 修复位置 | 新增测试 |
|------|------|---------|---------|
| HIGH | `page == 0` 未拒绝负数，`OFFSET -40` 导致 PostgreSQL 500 | `service.rs:104`，改为 `page < 1` | SV-09、SV-10 |
| MEDIUM | `mute_type` 未做枚举白名单，非法值静默返回空结果 | `service.rs:113-119`，添加白名单校验 | SV-11~SV-16 |
| LOW | `resolve_params` 返回 5-元组末位重复 `limit` | `service.rs:73`，简化为 4-元组 | — |

---

## 十、相关文档

- [TDS 技术设计](../../tds/adminServer/T-10016.md) — 完整接口契约与 Review 记录
- [审计日志模块 (T-10012)](./audit.md) — AuditLogger 实现细节
- [RBAC 权限中间件 (T-10003)](./rbac.md) — Permission 枚举定义
- [Analytics 查询模块 (T-10015)](./analytics.md) — 类似查询模式参考
- [Admin Server 总索引](./index.md)
