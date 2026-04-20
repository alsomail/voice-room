<!--
[AI 读写指令]
1. 本文件记录 T-10002（管理员登录接口）的架构实现细节，由 DoD Agent 生成。
2. 禁止在此文件手写业务逻辑代码，所有内容须从实际代码提取。
3. 后续修改（如新增 RBAC 中间件）须同步更新本文件对应章节。
-->

# Auth 模块架构文档（管理员登录）

**Last Updated:** 2026-04-20  
**Task ID:** T-10002  
**Entry Points:** `app/adminServer/src/modules/auth/`  
**关联 TDS:** [T-10002 TDS](../../tds/adminServer/T-10002.md)

---

## 一、路由注册

| 方法 | 路径 | Handler | 鉴权 |
|------|------|---------|------|
| `POST` | `/api/v1/admin/login` | `login_handler` | 无（公开接口） |

路由在 `app/adminServer/src/bootstrap/mod.rs` 的 `build_app()` 中注册：

```rust
Router::new()
    .route("/api/v1/admin/login", post(login_handler))
    .layer(middleware::from_fn(request_context_middleware))
    .with_state(state)
```

---

## 二、请求 / 响应 DTO

### 请求体（`AdminLoginRequest`）

文件：`app/adminServer/src/modules/auth/dto.rs`

```rust
pub struct AdminLoginRequest {
    pub username: String,   // 管理员用户名
    pub password: String,   // 明文密码（bcrypt 校验）
}
```

> ⚠️ 遗留低优先级：`username` / `password` 无最大长度校验，bcrypt 对 >72 字节密码存在截断风险，计划在专项 Task 引入 `validator` crate 处理。

### 成功响应体（`AdminLoginResponse`）

```rust
pub struct AdminLoginResponse {
    pub token: String,          // JWT 字符串
    pub expires_in: u64,        // 固定 604800（7 天，单位：秒）
    pub admin: AdminInfo,       // 管理员基础信息
}

pub struct AdminInfo {
    pub id: String,                      // admin UUID（字符串形式）
    pub username: String,
    pub role: String,                    // operator / super_admin / cs / finance
    pub display_name: Option<String>,
    pub last_login_at: Option<String>,   // RFC 3339 格式时间戳
}
```

### HTTP 统一外层结构（`ApiResponse<T>`）

成功时响应体：
```json
{
  "code": 0,
  "data": { "token": "...", "expires_in": 604800, "admin": { ... } },
  "request_id": "uuid-or-caller-provided"
}
```

失败时响应体：
```json
{
  "code": 40106,
  "message": "Invalid admin credentials",
  "request_id": "uuid-or-caller-provided"
}
```

---

## 三、服务层（AdminAuthService）

文件：`app/adminServer/src/modules/auth/service.rs`

### 登录业务流程

```
POST /api/v1/admin/login
        │
        ▼
login_handler（提取 IP、解析 JSON）
        │
        ▼
AdminAuthService::login(username, password, ip_addr)
        │
        ├─ Step 1: AdminRepository::find_by_username(username)
        │         ├─ None → verify_password(password, DUMMY_HASH)  ← 时序保护
        │         │         └─ return Err(InvalidAdminCredentials)
        │         └─ Some(admin) → 继续
        │
        ├─ Step 2: verify_password(password, admin.password_hash)
        │         └─ false → return Err(InvalidAdminCredentials)
        │
        ├─ Step 3: admin.is_active == false → return Err(AccountDisabled)
        │
        ├─ Step 4: issue_admin_token(admin, jwt_secret)
        │         └─ AdminClaims { sub, role, iss, exp, iat }
        │
        ├─ Step 5: admin_repo.update_last_login_at(admin.id, now)
        │         └─ 失败仅 tracing::warn，不影响登录
        │
        └─ Step 6: log_repo.insert_login_log(admin.id, ip_addr)
                  └─ 失败仅 tracing::warn，不影响登录
```

### JWT 生成参数

| 字段 | 值 |
|------|----|
| `sub` | `admin.id`（UUID 字符串） |
| `role` | `admin.role`（operator / super_admin / cs / finance） |
| `iss` | `"voiceroom-admin"`（区别于 C 端 `"voiceroom"`） |
| `iat` | 当前 Unix 时间戳（秒） |
| `exp` | `iat + 604800`（7 天） |

签发调用：`voice_room_shared::jwt::token::encode_token(&claims, secret.as_bytes())`

---

## 四、时序攻击防护（DUMMY_HASH）

**文件：** `app/adminServer/src/modules/auth/service.rs`

**问题背景：** 账号不存在时若直接返回（< 1ms），而密码错误时执行 bcrypt（cost=12，约 200–400ms），攻击者可通过响应时间差枚举有效用户名。

**防护方案：** 使用预计算的 cost=12 有效 bcrypt 哈希常量 `DUMMY_HASH`，账号不存在时仍执行一次完整 bcrypt 计算：

```rust
const DUMMY_HASH: &str =
    "$2b$12$Xmta40fS.0LJFwy9lnGgUOM/QmkpJDiMt4ko7Qy15lxWmzhAzxeyC";

// 账号不存在时的恒定时间路径
None => {
    let _ = verify_password(password, DUMMY_HASH);
    return Err(AppError::InvalidAdminCredentials);
}
```

**验证：** 单元测试 `login_timing_protection_nonexistent_account_calls_bcrypt` 断言账号不存在路径耗时 ≥ 100ms。

---

## 五、Repository 层（数据库访问）

文件：`app/adminServer/src/modules/auth/repository.rs`

### Trait 抽象

```rust
#[async_trait]
pub trait AdminRepository: Send + Sync {
    async fn find_by_username(&self, username: &str) -> Result<Option<AdminModel>, AppError>;
    async fn update_last_login_at(&self, admin_id: Uuid, time: DateTime<Utc>) -> Result<(), AppError>;
}

#[async_trait]
pub trait AdminLogRepository: Send + Sync {
    async fn insert_login_log(&self, admin_id: Uuid, ip_address: Option<String>) -> Result<(), AppError>;
}
```

### PgAdminRepository（生产实现）

| 方法 | SQL | 说明 |
|------|-----|------|
| `find_by_username` | `SELECT ... FROM admins WHERE username = $1 AND deleted_at IS NULL` | 软删除过滤，参数化查询防注入 |
| `update_last_login_at` | `UPDATE admins SET last_login_at = $1, updated_at = $1 WHERE id = $2` | 同时更新 `updated_at` |

### PgAdminLogRepository（生产实现）

| 方法 | SQL | 说明 |
|------|-----|------|
| `insert_login_log` | `INSERT INTO admin_logs (admin_id, action, ip_address) VALUES ($1, 'admin_login', $2::inet)` | `ip_address` 字段类型为 INET，使用 `$2::inet` 显式转型 |

### Fake 实现（测试专用）

- `FakeAdminRepository`：内存 `HashMap`，提供 `seed(admin)` 和 `get_last_login_at(id)` 测试辅助方法
- `FakeAdminLogRepository`：内存 `Vec<LoginLogEntry>`，提供 `get_logs()` 测试辅助方法

---

## 六、登录日志写入逻辑

写入表：`admin_logs`  
触发时机：每次登录成功后（Step 6）  
写入字段：

| 字段 | 值 |
|------|----|
| `admin_id` | 登录管理员的 UUID |
| `action` | 固定字符串 `"admin_login"` |
| `ip_address` | 客户端 IP（类型 INET，可为 NULL） |

**IP 提取逻辑**（`app/adminServer/src/infrastructure/logging.rs`）：
```rust
// 优先 X-Forwarded-For（取第一个），次选 X-Real-IP
headers.get("x-forwarded-for")
    .or_else(|| headers.get("x-real-ip"))
    .map(|v| v.split(',').next().unwrap_or("").trim().to_string())
```

---

## 七、错误码映射

文件：`app/adminServer/src/common/error.rs`

| AppError 变体 | HTTP 状态码 | 业务错误码 |
|--------------|------------|-----------|
| `InvalidAdminCredentials` | 401 UNAUTHORIZED | `40106` |
| `AccountDisabled` | 403 FORBIDDEN | `40302` |
| `DatabaseError` | 500 INTERNAL_SERVER_ERROR | `50000` |
| `Internal` | 500 INTERNAL_SERVER_ERROR | `50000` |

> `InvalidAdminCredentials` 同时覆盖"账号不存在"和"密码错误"两种场景，统一返回 401/40106，防止用户名枚举。

---

## 八、请求上下文中间件

文件：`app/adminServer/src/infrastructure/logging.rs`

`request_context_middleware` 应用于全局路由，职责：

1. 提取 `X-Request-Id` 请求头（不存在则生成新 UUIDv4）
2. 注入 `RequestContext` 到 Axum Extension（供 handler 读取 `request_id`）
3. 绑定 tracing `info_span`（含 request_id、method、uri、status_code）
4. 在响应头回传 `X-Request-Id`

---

## 九、应用启动流程（main.rs）

文件：`app/adminServer/src/main.rs`

```
启动顺序：
1. dotenvy::dotenv()           — 加载 .env（生产环境忽略）
2. tracing_subscriber::fmt     — 初始化日志（RUST_LOG 控制级别）
3. env::var("DATABASE_URL")    — 必填环境变量
4. env::var("JWT_SECRET")      — 必填环境变量
5. env::var("PORT")            — 可选，默认 8081
6. PgPool::connect()           — 初始化 PostgreSQL 连接池
7. sqlx::migrate!()            — 运行 ./migrations 目录迁移
8. AppState::new(              — 注入真实 PgRepository
       PgAdminRepository,
       PgAdminLogRepository,
       jwt_secret
   )
9. build_app(state)            — 构建 Axum Router
10. axum::serve(...).with_graceful_shutdown() — 启动 HTTP 服务（支持 SIGTERM/Ctrl-C）
```

---

## 十、测试覆盖

| 测试类型 | 数量 | 文件 | 覆盖场景 |
|---------|------|------|---------|
| 单元测试（Service） | 10 | `service.rs` | U01~U09 + 时序保护 |
| 集成测试（HTTP） | 7 | `bootstrap/mod.rs` | I-01~I-07（HTTP 状态码+错误码+响应结构） |
| 单元测试（Error） | 4 | `common/error.rs` | 错误码映射验证 |
| doctest | 1 | — | — |
| **合计** | **41+** | — | — |

关键测试场景：
- `login_timing_protection_nonexistent_account_calls_bcrypt` — 验证 DUMMY_HASH 时序保护（耗时 ≥ 100ms）
- `post_login_records_client_ip_from_x_forwarded_for` — 验证 IP 提取逻辑
- `login_response_header_contains_request_id` — 验证 X-Request-Id 回传

---

## 十一、依赖关系

```
login_handler
    └── AdminAuthService
            ├── AdminRepository (trait)
            │       └── PgAdminRepository (SQLx + PostgreSQL)
            ├── AdminLogRepository (trait)
            │       └── PgAdminLogRepository (SQLx + PostgreSQL)
            └── voice_room_shared
                    ├── crypto::verify_password (bcrypt)
                    ├── jwt::token::encode_token
                    └── models::AdminModel
```

## 十二、相关文档

- [Admin Server 架构总索引](./index.md)
- [管理员数据层（T-10001）](./admins-table.md) — admins / admin_logs 表 DDL
- [T-10002 TDS](../../tds/adminServer/T-10002.md)
- [接口协议（protocol.md）](../../protocol.md) §三 3.1
