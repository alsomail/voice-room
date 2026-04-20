<!--
[AI 读写指令]
1. 本文件记录 Admin Server stats 模块（数据统计）的架构与实现细节。
2. 对应 Task: T-10010
-->

# Stats 模块 — 数据统计接口

## 一、模块定位

`src/modules/stats/` 提供运营数据统计概览接口，供运营人员查看平台核心指标（DAU、新增用户、活跃房间数、在线人数）。

## 二、目录结构

```text
src/modules/stats/
├── mod.rs           # 模块声明与 pub use 导出
├── dto.rs           # StatsOverviewQuery / DateRange / StatsOverviewResponse
├── repository.rs    # AdminStatsRepository trait + PgAdminStatsRepository + FakeAdminStatsRepository
├── service.rs       # AdminStatsService::get_overview（日期解析、并发查询、MVP mock）
└── controller.rs    # stats_overview_handler
```

## 三、接口规格

```
GET /api/v1/admin/stats/overview?start_date=2024-01-01&end_date=2024-01-31
Authorization: Bearer <admin-jwt>
```

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `start_date` | string (YYYY-MM-DD) | 否 | 统计起始日期，缺省今天（UTC） |
| `end_date` | string (YYYY-MM-DD) | 否 | 统计截止日期，缺省今天（UTC） |

### 成功响应（200）

```json
{
  "code": 0,
  "message": "ok",
  "request_id": "xxx",
  "data": {
    "dau": 1234,
    "new_users": 56,
    "active_rooms": 0,
    "online_users": 0,
    "date_range": { "start": "2024-01-01", "end": "2024-01-31" }
  }
}
```

| 字段 | 来源 | 说明 |
|------|------|------|
| `dau` | DB users.updated_at | updated_at 在区间内的未删除用户数（近似 DAU） |
| `new_users` | DB users.created_at | created_at 在区间内的未删除用户数 |
| `active_rooms` | **MVP mock = 0** | T-10011 接入 Redis `SCARD rooms:active` 后替换 |
| `online_users` | **MVP mock = 0** | T-10011 接入 Redis `SCARD online:users` 后替换 |

### 错误码

| HTTP | code | 场景 |
|------|------|------|
| 400 | 40003 | 日期格式非法或 start_date > end_date |
| 401 | 40101 | 无 Token / Token 无效 |
| 403 | 40301 | cs 角色无 StatsRead 权限 |
| 500 | 50000 | 数据库内部错误 |

## 四、权限

**Permission::StatsRead** — 已在 `common/auth/context.rs` 定义，无需新增。

| 角色 | 可访问 |
|------|--------|
| super_admin | ✅ |
| operator | ✅ |
| finance | ✅ |
| cs | ❌（403） |

## 五、Repository 抽象

`AdminStatsRepository` trait 隔离 DB 与测试：

```rust
#[async_trait]
pub trait AdminStatsRepository: Send + Sync {
    async fn count_new_users(&self, start: NaiveDate, end: NaiveDate) -> Result<i64, AppError>;
    async fn count_dau(&self, start: NaiveDate, end: NaiveDate) -> Result<i64, AppError>;
}
```

- **PgAdminStatsRepository**：通过 SQLx 参数化查询访问 users 表（`$1`/`$2` 占位符，无 SQL 注入风险）
- **FakeAdminStatsRepository**：`HashMap<(NaiveDate, NaiveDate), i64>` 内存实现，用于单元测试

## 六、Service 核心逻辑

`AdminStatsService::get_overview` 流程：
1. 解析 start_date / end_date（缺省今天 UTC，格式非法 → 400/40003）
2. 校验 start <= end（否则 → 400/40003）
3. `tokio::try_join!(repo.count_new_users, repo.count_dau)` 并发查询
4. active_rooms / online_users MVP 固定为 0（含 TODO(T-10011) 注释）
5. 返回 StatsOverviewResponse

## 七、测试覆盖

| 类型 | 用例 | 状态 |
|------|------|------|
| Repository 单元测试 | RT-01~03（FakeRepo 预置值、默认零值） | ✅ |
| Service 单元测试 | ST-01~06（正常查询、缺省日期、格式错误、start>end） | ✅ |
| HTTP 集成测试 | US-01~05（200 正常、403 cs、401 无 token、400 非法日期） | ✅ |

**总新增测试数：14 条**（原 206 → 220）

## 八、已知限制与后续优化

| 项目 | 说明 | 跟进 Task |
|------|------|-----------|
| active_rooms / online_users 固定为 0 | App Server Redis 未接入 | T-10011 |
| 无查询天数上限（建议 ≤ 366 天） | 极端范围可触发全表 COUNT | T-10011 |
| 使用 UTC 日期，与 UTC+4 本地日期有偏差 | 运营侧需知晓 | 待定 |
