<!--
[AI 读写指令]
1. 本文件由 DoD Agent 生成，记录 T-10003（管理员 JWT 中间件 + RBAC）的架构实现细节。
2. 禁止在此文件手写业务逻辑代码，所有内容须从实际代码提取。
3. 后续扩展角色或权限时，须同步更新【三、RBAC 角色权限矩阵】和【四、Permission 枚举】两节。
-->

# RBAC 权限中间件架构文档

**Last Updated:** 2026-04-21  
**Task ID:** T-10003  
**Entry Points:**
- `app/adminServer/src/common/auth/context.rs`
- `app/adminServer/src/common/middleware/jwt_auth.rs`  

**关联 TDS:** [T-10003 TDS](../../tds/adminServer/T-10003.md)

---

## 一、整体数据流

```
HTTP Request
    │
    ▼
request_context_middleware          ← 注入 RequestContext（含 request_id）
    │
    ▼
AdminAuthContext::from_request_parts（FromRequestParts<AppState>）
    │
    ├─ 提取 Authorization: Bearer <token>
    │       └─ 缺失 / 格式错误 → 401 (40101)
    │
    ├─ decode_token(token, jwt_secret, "voiceroom-admin")
    │       ├─ 签名无效 / iss ≠ "voiceroom-admin" → 401 (40101)
    │       └─ 已过期 (ErrorKind::ExpiredSignature) → 401 (40102)
    │
    ├─ Uuid::parse_str(claims.sub)
    │       └─ 非 UUID 格式 → 401 (40101)
    │
    └─ Ok(AdminAuthContext { admin_id, role })
            │
            ▼
    Handler 提取 AdminAuthContext
            │
            ▼
    ctx.require_permission(Permission::XxxYyy)
            ├─ has_permission == true  → Ok(())  → 继续执行
            └─ has_permission == false → Err(AppError::Forbidden) → 403 (40301)
```

---

## 二、AdminAuthContext 结构体

**文件：** `app/adminServer/src/common/auth/context.rs`

```rust
#[derive(Clone, Debug)]
pub struct AdminAuthContext {
    pub admin_id: Uuid,    // 从 JWT claims.sub 解析，类型安全的 UUID
    pub role: String,      // 原始角色字符串："super_admin" | "operator" | "cs" | "finance"
}
```

### 核心方法

| 方法签名 | 说明 |
|---------|------|
| `new(admin_id: Uuid, role: impl Into<String>) -> Self` | 构造函数 |
| `has_permission(&self, permission: Permission) -> bool` | 查询当前角色是否持有指定权限（不报错） |
| `require_permission(&self, permission: Permission) -> Result<(), AppError>` | 断言权限，不足时返回 `AppError::Forbidden`（HTTP 403 / 40301） |

---

## 三、Permission 枚举

**文件：** `app/adminServer/src/common/auth/context.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    // 用户管理
    UserRead,       // 查询用户列表 / 详情
    UserWrite,      // 封禁 / 解封用户

    // 房间管理
    RoomRead,       // 查询房间列表 / 详情
    RoomWrite,      // 通用房间写入（预留，当前无端点使用）
    RoomForceClose, // 强制关闭房间（DELETE /admin/rooms/:id）

    // 数据统计
    StatsRead,      // 查看统计概览

    // 财务操作
    FinanceRead,    // 查看财务数据
    FinanceWrite,   // 财务操作（充值/退款等）

    // 系统管理
    SystemAdmin,    // 系统级管理（仅 super_admin）
    LogRead,        // 查看审计日志（GET /admin/logs）
}
```

---

## 四、RBAC 角色权限矩阵

> 对应 `has_permission` 方法的 `match self.role.as_str()` 分支，与 `doc/protocol/admin_api.md §4.3` 完全一致。

| Permission | super_admin | operator | cs（客服） | finance |
|-----------|:-----------:|:--------:|:---------:|:-------:|
| `UserRead` | ✅ | ✅ | ✅ | ❌ |
| `UserWrite` | ✅ | ✅ | ❌ | ❌ |
| `RoomRead` | ✅ | ✅ | ✅ | ❌ |
| `RoomWrite` | ✅ | ✅ | ❌ | ❌ |
| `RoomForceClose` | ✅ | ✅ | ❌ | ❌ |
| `StatsRead` | ✅ | ✅ | ❌ | ✅ |
| `FinanceRead` | ✅ | ❌ | ❌ | ✅ |
| `FinanceWrite` | ✅ | ❌ | ❌ | ✅ |
| `SystemAdmin` | ✅ | ❌ | ❌ | ❌ |
| `LogRead` | ✅ | ✅ | ❌ | ❌ |

**矩阵说明：**
- `super_admin`：`has_permission` 恒返回 `true`（全权）
- `operator`：用户读写 + 房间读写 + 强制关房 + 数据统计 + 日志读，无财务和系统权限
- `cs`（客服）：用户只读 + 房间只读，无写入/统计/财务/系统权限
- `finance`：统计读 + 财务读写，无用户/房间/系统管理权限
- 未知角色：恒返回 `false`（fail-closed 安全策略）

---

## 五、FromRequestParts 中间件实现

**文件：** `app/adminServer/src/common/middleware/jwt_auth.rs`

### 实现方式

`AdminAuthContext` 直接实现 Axum 的 `FromRequestParts<AppState>` trait，作为 Handler 参数自动提取，**无需注册为 Tower 中间件层**。

```rust
impl FromRequestParts<AppState> for AdminAuthContext {
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> { ... }
}
```

### request_id 透传机制

`from_request_parts` 不直接生成 request_id，而是从 Axum Extensions 中读取由 `request_context_middleware`（全局中间件）预先注入的 `RequestContext`：

```rust
let request_id = parts
    .extensions
    .get::<RequestContext>()          // ← 由 request_context_middleware 注入
    .map(|rc| rc.request_id().to_string())
    .unwrap_or_default();             // 降级为空字符串，不崩溃
```

提取到的 `request_id` 传入 `AppError::into_rejection_with_id(&request_id)`，确保**鉴权拒绝响应也携带 request_id**，满足 `protocol §1.3` 的全链路追踪要求。

### 纯函数 extract_admin_auth_context

提取逻辑被拆成独立纯函数，解耦 Axum 框架依赖，便于单元测试：

```rust
pub fn extract_admin_auth_context(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<AdminAuthContext, AppError>
```

---

## 六、错误码映射

**文件：** `app/adminServer/src/common/error.rs`

| AppError 变体 | HTTP 状态码 | 业务错误码 | 触发场景 |
|--------------|:-----------:|:---------:|---------|
| `Unauthorized` | 401 | `40101` | 无 Header / 非 Bearer 格式 / 签名无效 / iss 错误 / sub 非 UUID |
| `TokenExpired` | 401 | `40102` | JWT `exp` 已过期（`ErrorKind::ExpiredSignature`） |
| `Forbidden` | 403 | `40301` | `require_permission` 校验不通过 |

**响应体格式（鉴权失败）：**

```json
{
  "code": 40101,
  "message": "Unauthorized",
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

> ⚠️ 遗留项（不阻塞）：`IntoResponse for AppError` 的默认路径 `request_id` 为空字符串，handler 需手动调用 `err_response(e, request_id)`。鉴权中间件路径已通过 `into_rejection_with_id` 正确透传。

---

## 七、测试覆盖

### 单元测试（context.rs）— 12 个

| 测试 ID | 测试名称 | 验证场景 |
|--------|---------|---------|
| T-10003-R01 | `super_admin_has_all_permissions` | super_admin 持有全部 8 项权限 |
| T-10003-R02 | `operator_has_user_room_stats_permissions` | operator 持有用户/房间/统计 5 项权限 |
| T-10003-R03 | `operator_lacks_finance_permissions` | operator 无 FinanceRead/FinanceWrite |
| T-10003-R04 | `operator_lacks_system_admin` | operator 无 SystemAdmin |
| T-10003-R05 | `cs_has_user_read_and_room_read_permissions` | cs 持有 UserRead + RoomRead |
| T-10003-R06 | `cs_lacks_user_write` | cs 无 UserWrite（不能封禁） |
| T-10003-R07 | `cs_lacks_room_write_stats_finance_system` | cs 无 RoomWrite/统计/财务/系统 5 项权限 |
| T-10003-R08 | `finance_has_stats_and_finance_permissions` | finance 持有 StatsRead + 财务读写 |
| T-10003-R09 | `finance_lacks_user_room_system_permissions` | finance 无用户/房间/系统 5 项权限 |
| T-10003-R10 | `unknown_role_has_no_permissions` | 未知角色无任何权限 |
| T-10003-R11 | `require_permission_ok_when_allowed` | 有权限时返回 `Ok(())` |
| T-10003-R12 | `require_permission_err_when_denied` | 无权限时返回 `Err(AppError::Forbidden)` |

### 单元测试（jwt_auth.rs）— 8 个

| 测试 ID | 测试名称 | 验证场景 |
|--------|---------|---------|
| T-10003-J01 | `missing_auth_header_returns_unauthorized` | 无 Authorization Header → Unauthorized |
| T-10003-J02 | `invalid_bearer_format_returns_unauthorized` | "Token xxx" 非 Bearer 格式 → Unauthorized |
| T-10003-J03 | `invalid_signature_returns_unauthorized` | 签名错误（wrong secret）→ Unauthorized |
| T-10003-J04 | `expired_token_returns_token_expired` | exp 已过期 → TokenExpired |
| T-10003-J05 | `valid_admin_token_injects_admin_id_and_role` | 合法 token → 正确注入 admin_id & role |
| T-10003-J06 | `app_token_with_wrong_iss_returns_unauthorized` | C 端 iss="voiceroom" → Unauthorized |
| T-10003-J07 | `non_uuid_sub_returns_unauthorized` | sub 非 UUID 格式 → Unauthorized |
| T-10003-J08 | `operator_token_injects_correct_role` | operator token → role 字段正确 |

### 集成测试（bootstrap/mod.rs）— 9 个

I-08 ~ I-16，覆盖全部 RBAC 场景（含 super_admin 全权、operator 权限边界、cs 只读约束、finance 财务专属、403 场景等）。

**合计：74 个测试全部通过**（含 T-10001 / T-10002 历史用例）

---

## 八、依赖关系

```
AdminAuthContext::from_request_parts
    │
    ├── extract_admin_auth_context(headers, jwt_secret)
    │       └── voice_room_shared::jwt::token::decode_token
    │               └── jsonwebtoken crate（iss 校验内置）
    │
    ├── RequestContext（来自 request_context_middleware）
    │       └── AppError::into_rejection_with_id(request_id)
    │
    └── AdminAuthContext::has_permission / require_permission
            └── Permission 枚举（RBAC 矩阵）
```

---

## 九、相关文档

- [Admin Server 架构总索引](./index.md)
- [Auth 模块（T-10002）](./auth.md) — 管理员登录、JWT 签发
- [管理员数据层（T-10001）](./admins-table.md) — admins / admin_logs 表 DDL
- [T-10003 TDS](../../tds/adminServer/T-10003.md)
- [接口协议（protocol.md）](../../protocol.md) §三 3.3（RBAC 权限矩阵）
