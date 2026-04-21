<!--
[AI 读写指令]
1. 本文件由 Plan Agent 创建，记录 T-10013 手动调整余额 API 的架构设计
2. TDD 阶段完成实现后，在【二、API 设计】中更新实际实现的代码片段
3. DoD 阶段同步文档状态后，在【四、状态检查清单】更新完成时间戳
-->

# Admin Server 钱包模块 (Wallet Module) - T-10013

**最后更新**: 2025-07-16 (DoD 完成)  
**负责人**: Dod  
**状态**: ✅ 已完成

---

## 一、模块概述

### 功能定位
Admin Server 的 Wallet 模块提供后台管理员为用户**手动调整钻石余额**的能力，支持加/扣钻石、审计日志、Redis 事件发布。

### 核心特性
- **事务性操作**：单个数据库事务内完成余额更新、流水记录、审计日志写入
- **Redis 事件发布**：余额变更后通过 Redis Pub/Sub 通知 App Server，触发客户端 WS 推送
- **RBAC 权限控制**：仅限 `super_admin` / `operator` / `finance` 角色操作
- **失败保护**：余额不足防护、金额范围检查、原因字段必填

### 关联 Task
- **上游依赖**: T-00017 (钱包 Schema 初始化)、T-10012 (审计日志模块)
- **下游消费者**: T-00018 (App Server 余额推送 WS)、T-20012 (Web 后台调整 UI)

---

## 二、API 设计

### 接口定义

**请求**:
```http
POST /api/v1/admin/users/{id}/wallet/adjust HTTP/1.1
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "amount": 1000,
  "reason": "运营补偿：客诉处理 #1234"
}
```

**参数说明**:
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `amount` | i32 | 非零、±≤10,000,000 | 正数=加钻石，负数=扣钻石 |
| `reason` | string | 2-200字符 | 调整原因，必填，记入 admin_logs |

**响应 200** (成功):
```json
{
  "code": 0,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "new_balance": 5234,
    "delta": 1000
  },
  "request_id": "req-xxx"
}
```

**错误响应**:

| HTTP | code | 说明 |
|------|------|------|
| 400 | 40003 | amount=0 / reason 不足 2 字符 / reason 超过 200 字符 / 金额绝对值超 1000 万 |
| 401 | 40101 | 未认证（无 JWT） |
| 401 | 40102 | JWT 过期 |
| 403 | 40301 | role=cs（无 WalletAdjust 权限） |
| 404 | 40400 | 用户不存在或已删除 |
| 400 | 40204 | 调整后余额 < 0 |

---

## 三、核心数据流

### 3.1 事务流程

```
POST /api/v1/admin/users/:id/wallet/adjust
  ↓
[RBAC 中间件] 校验 JWT + 权限 (WalletAdjust)
  ↓ 若失败 → 403 / 401
[参数校验]
  - amount ≠ 0
  - reason.len() ∈ [2, 200]
  - |amount| ≤ 10,000,000
  ↓ 若失败 → 400 (40003)
[事务开启] BEGIN TRANSACTION
  ↓
[Step 1] SELECT users WHERE id = ? FOR UPDATE
  ↓ 若不存在或已删除 → 404 (40400)，事务回滚
[Step 2] new_balance = current_balance + amount
  ↓ 若 new_balance < 0 → 400 (40204)，事务回滚
[Step 3] UPDATE users SET diamond_balance = new_balance WHERE id = ?
[Step 4] INSERT wallet_transactions (
    user_id,
    type = 'admin_adjust',
    amount,
    balance_after = new_balance,
    operator_id = <from JWT>,
    reason,
    created_at = now()
  )
[Step 5] INSERT admin_logs (
    admin_id = <from JWT>,
    action = 'wallet_adjust',
    target_type = 'user',
    target_id = user_id,
    detail = {
      amount,
      reason,
      new_balance,
      delta = amount
    },
    ip = <from request>,
    created_at = now()
  )
  ↓ 若任何步骤异常 → ROLLBACK，无副作用
[事务提交] COMMIT
  ↓
[Redis 发布] PUBLISH admin:events {
    type: "balance_updated",
    payload: {
      user_id,
      new_balance,
      delta: amount,
      reason
    },
    admin_id: <from JWT>,
    ts: unix_timestamp()
  }
  ↓ 发布失败仅 warn，不影响主业务
[响应] 200 { new_balance, delta, user_id }
```

### 3.2 权限矩阵

| 角色 | WalletAdjust | 说明 |
|------|-------------|------|
| super_admin | ✅ | 全权限 |
| operator | ✅ | 可调整 |
| finance | ✅ | 财务可调整 |
| cs | ❌ | 客服无权 (403) |

---

## 四、模块结构

```text
app/adminServer/src/modules/wallet/
├── mod.rs                 # 模块入口、pub use
├── dto.rs                 # AdjustBalanceRequest / AdjustBalanceResponse
├── handler.rs             # adjust_balance_handler、参数校验
├── service.rs             # WalletService、事务+发布
├── repository.rs          # WalletRepository trait、PgWalletRepository、FakeWalletRepository
└── [集成测试]
    └── tests/wallet_test.rs # WA01~WA08 验收用例
```

### 4.1 关键数据库操作

**涉及表**:
- `users` — UPDATE diamond_balance
- `wallet_transactions` — INSERT (type='admin_adjust')
- `admin_logs` — INSERT (action='wallet_adjust')

**索引策略**:
- `users` 表在事务内使用 `FOR UPDATE` 行级锁，防止并发数据不一致
- `wallet_transactions` 按 `user_id + created_at DESC` 查询历史

---

## 五、错误码映射

| ErrorCode | HTTP 状态 | 说明 |
|-----------|----------|------|
| `InsufficientBalance` (40204) | 400 | 调整后余额 < 0 |
| `ValidationError` (40003) | 400 | 参数校验失败 |
| `NotFound` (40400) | 404 | 用户不存在 |
| `Unauthorized` (40101/40102) | 401 | JWT 无效/过期 |
| `Forbidden` (40301) | 403 | 权限不足 |

---

## 六、验收用例

### TDD 验收标准 (WA01~WA08)

| 用例 | 操作 | 期望结果 | 断言 |
|------|------|---------|------|
| **WA01** | super_admin 调用，+1000 | 200，余额 +1000 | 余额已更新、流水 +1、admin_logs +1、Redis 已发布 |
| **WA02** | cs 角色调用 | 403 | 请求被拒绝 |
| **WA03** | amount=0 | 400 (40003) | 参数校验失败 |
| **WA04** | reason="" | 400 (40003) | reason 不足 2 字符 |
| **WA05** | 用户 id 不存在 | 404 (40400) | 用户查询返回空 |
| **WA06** | 余额 500，amount=-1000 | 400 (40204) | 事务回滚，余额不变 |
| **WA07** | amount=20,000,000 | 400 (40003) | 金额绝对值超限 |
| **WA08** | admin_logs INSERT 异常 | 400 | 整个事务回滚，余额保持原值 |

---

## 七、外部依赖

| 依赖 | 来源 | 用途 |
|-----|------|------|
| `redis::RedisClient` | infrastructure::cache | Redis Pub/Sub 发布 |
| `sqlx::PgPool` | infrastructure::db | 数据库事务 |
| `AdminAuthContext` | common::auth | JWT 解析与权限检查 |
| `EventPublisher` trait | modules::event (T-10011) | 事件发布门面 |
| `AuditLogger` | modules::audit (T-10012) | 审计日志记录 |

---

## 八、状态检查清单

- [x] TDS 文档完成（doc/tds/adminServer/T-10013.md）
- [x] 代码实现完成 (27 新增测试，全量 286 tests passed)
- [x] Clippy 零警告
- [x] 事务原子性验证 (WA08 通过)
- [x] RBAC 权限矩阵验证 (WA02 通过)
- [x] 余额防护验证 (WA06 通过)
- [x] Redis 事件发布验证 (WA01 通过)
- [x] 架构文档已同步 (2025-07-16)

---

## 九、相关文档

- **TDS 完整设计**: [doc/tds/adminServer/T-10013.md](../../tds/adminServer/T-10013.md)
- **RBAC 权限体系**: [rbac.md](./rbac.md)
- **事件发布模块**: [event.md](./event.md) ← T-10011
- **审计日志模块**: [audit.md](./audit.md) ← T-10012
- **产品方向**: [doc/product/phase1_gift_economy.md §7](../../product/phase1_gift_economy.md)
