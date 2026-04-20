<!--
[AI 读写指令与维护规约]
1. 本文件记录 Auth 模块的架构与实现细节，由 DoD Agent 在 T-00002 完成后首次生成。
2. 后续 T-00003 / T-00004 / T-00005 完成时，需对应更新本文件的接口列表与能力状态。
3. 禁止在此文件中直接粘贴大段代码；以文件路径 + 行为描述为主。
-->

# Auth 模块架构文档

**Last Updated:** 2025-07-16
**关联 Task:** T-00002（短信验证码发送）、T-00003（手机号登录）、T-00004（JWT 中间件）、T-00005（获取用户信息）
**入口文件:** `app/server/src/modules/auth/routes.rs`

---

## 一、模块结构

```
src/
├── modules/auth/
│   ├── routes.rs       # Axum 路由注册，auth_routes() → Router<AppState>
│   ├── controller.rs   # Handler 层，负责提取 Extension/State、调用 Service、统一响应
│   ├── service.rs      # AuthService：业务逻辑（send_code / login / get_me）
│   ├── dto.rs          # 请求 / 响应 DTO（Serialize/Deserialize）
│   ├── repository.rs   # UserRepository trait + PgUserRepository（SQLx）+ FakeUserRepository
│   └── mod.rs          # 对外 re-export routes.rs
│
├── infrastructure/
│   ├── redis_store/
│   │   └── mod.rs      # SmsCodeStore trait + RedisCodeStore + FakeCodeStore
│   └── third_party/sms/
│       ├── mod.rs      # SmsProvider trait + re-export
│       ├── twilio.rs   # TwilioSmsProvider（生产实现）
│       └── mock.rs     # MockSmsProvider（开发/CI）、FailingSmsProvider（测试异常路径）
│
└── common/
    ├── error.rs        # AppError 枚举、err_response()、safe_message()
    ├── response.rs     # ApiResponse<T> 统一成功响应结构
    └── auth/           # AuthContext（JWT 鉴权 Extractor，T-00004）
```

---

## 二、已注册路由

| Method | Path | Handler | Task | 状态 |
|--------|------|---------|------|------|
| `POST` | `/api/v1/auth/verification-codes` | `send_code` | T-00002 | 🟢 完成 |
| `POST` | `/api/v1/auth/login` | `login` | T-00003 | 🟢 完成 |
| `GET`  | `/api/v1/users/me` | `get_me` | T-00005 | 🟢 完成 |

---

## 三、T-00002 核心数据流：发送短信验证码

```
POST /api/v1/auth/verification-codes  {"phone": "+8613800138000"}
        │
        ▼ controller::send_code
        │  extract State(AppState) + Extension(RequestContext) + Json(SendCodeRequest)
        │
        ▼ AuthService::send_code(phone)
        │  1. validate_phone(phone)       ← E.164 格式：+<6-14位数字>
        │  2. generate_code()             ← rand 6位数字，格式化为 {:06}
        │  3. code_store.save_code()      ← Lua 原子脚本（检查冷却/日限 → 写入）
        │     ├─ VR:COOLDOWN   → AppError::VerificationCodeCooldown  (HTTP 429 / 42901)
        │     └─ VR:DAILY_LIMIT → AppError::VerificationCodeDailyLimit (HTTP 429 / 42902)
        │  4. sms.send_verification_code() ← SmsProvider 防腐层
        │     └─ 失败 → code_store.revoke_code()（清除 code + cooldown，保留 daily count）
        │              → 返回 AppError::SmsSendFailed (HTTP 500)
        │
        ▼ 成功 → SendCodeResponse { expires_in: 300, cooldown: 60 }
        ▼ 响应 → ApiResponse<SendCodeResponse> + request_id（来自 RequestContext）
```

### Save-First 并发模型

`send_code` 采用"先写 Redis 占位，再发 SMS，失败则撤销"方案：

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | `save_code` (Lua) | 原子检查冷却期 & 日限，通过则写入 code + cooldown + daily incr |
| 2 | `sms.send()` | 发送短信；此阶段 Redis 已有占位，同手机号并发请求被 Lua 拒绝 |
| 3a | 成功 | 返回 `{expires_in: 300, cooldown: 60}` |
| 3b | 失败 | `revoke_code` 删除 code_key + cooldown_key；daily count **保留**（防滥用）|

---

## 四、T-00003 核心数据流：手机号一键登录

```
POST /api/v1/auth/login  {"phone": "+8613800138000", "code": "123456"}
        │
        ▼ controller::login
        │  extract State(AppState) + Extension(RequestContext) + Json(LoginRequest)
        │
        ▼ AuthService::login(phone, code)
        │  1. validate_phone(phone)            ← E.164 格式校验（同 T-00002）
        │  2. code_store.verify_and_consume()  ← VERIFY_CODE_LUA 原子消费（见下）
        │     ├─ VR:EXPIRED      → AppError::VerificationCodeExpired     (HTTP 401 / 40104)
        │     ├─ VR:MAX_ATTEMPTS → AppError::VerificationCodeMaxAttempts (HTTP 401 / 40105)
        │     └─ VR:INVALID      → AppError::InvalidVerificationCode     (HTTP 401 / 40103)
        │  3. user_repo.find_by_phone(phone)   ← SELECT … WHERE phone=$1 AND deleted_at IS NULL
        │     ├─ Some(user) → is_new = false
        │     └─ None       → nickname = "User{手机末4位}"
        │                      user_repo.create(phone, &nickname) → is_new = true
        │  4. 封禁检查：user.is_banned == true  → AppError::Unauthorized (HTTP 401 / 40101)
        │  5. issue_token(user, jwt_secret)    ← encode_token(AppClaims, secret)
        │     └─ now = now_secs(); exp = now + 2592000; iss = "voiceroom"
        │
        ▼ 成功 → LoginResponse { token, expires_in: 2592000, user: LoginUserInfo }
        ▼ 响应 → ApiResponse<LoginResponse> + request_id（来自 RequestContext）
```

### OTP 原子消费机制（VERIFY_CODE_LUA）

`verify_and_consume` 通过单个 Lua 脚本原子完成"校验 + 消费"，防止同一 OTP 被重复使用：

| 步骤 | Lua 操作 | 说明 |
|------|---------|------|
| 1 | `HGETALL sms:code:{phone}` | 读取 code、attempts 字段 |
| 2 | key 不存在 | 验证码已过期或已被消费 → `VR:EXPIRED` (40104) |
| 3 | attempts ≥ 5 | 错误尝试达上限 → `VR:MAX_ATTEMPTS` (40105) |
| 4 | code 不匹配 | `HINCRBY attempts +1` → `VR:INVALID` (40103) |
| 5 | code 匹配 | `DEL sms:code:{phone}` — **原子删除，OTP 立即失效** |

> **并发安全**：Lua 脚本在 Redis 单线程执行，并发两个相同请求只有一个能通过 DEL；第二个请求读到 key 已不存在，返回 40104。已有 `redis_store` 双重消费防护测试覆盖。

### 新用户自动注册逻辑

```
find_by_phone(phone)
    ├─ Some(user) → is_new = false，直接进入封禁检查
    └─ None       → suffix   = phone 末 4 位数字
                    nickname = format!("User{suffix}")   // 例："+8613800138000" → "User8000"
                    user_repo.create(phone, &nickname)
                        └─ INSERT INTO users(phone, nickname) RETURNING *
                           并发冲突兜底：DB 的 phone UNIQUE 约束（T-00001 migration）
                    is_new = true
```

> **注**：昵称格式实现为 `"User{末4位数字}"`（如 `"User8000"`），与 `protocol.md §2.2` 示例（`"User_a1b2"`）略有差异，待 PM 确认后对齐（T-00003 第三轮 Review 遗留观察，不阻塞）。

### JWT 生成流程

```
issue_token(user, jwt_secret)          // service.rs
  ├─ now = SystemTime::now() → Unix 秒（单次调用，消除 iat/exp 偏差）
  ├─ claims = AppClaims {
  │      sub: user.id (UUID string),
  │      iss: "voiceroom",
  │      exp: now + 2592000,           // 30 天
  │      iat: now,
  │  }
  └─ encode_token(&claims, secret.as_bytes())  // shared::jwt::token::encode_token
```

- `jwt_secret` 来自环境变量注入（`AppState` 构造时传入），无硬编码
- `expires_in: 2592000`（30 天秒数）随 `LoginResponse` 一并返回给客户端
- `LoginResponse` DTO 字段：`token / expires_in / user(id/phone/nickname/avatar/coin_balance/vip_level/is_new/created_at)`

---

## 五、T-00005 核心数据流：获取用户信息

```
GET /api/v1/users/me  (Authorization: Bearer <token>)
        │
        ▼ AuthContext::from_request_parts（JWT 鉴权 Extractor，common/middleware/jwt_auth.rs）
        │  1. 从 parts.extensions 提取 RequestContext，获取真实 request_id
        │     └─ 保证 JWT 认证失败时错误响应携带正确 request_id（protocol §1.3 合规）
        │  2. extract_auth_context(&parts.headers, &state.jwt_secret)
        │     ├─ 无 Authorization header  → AppError::Unauthorized  (401 / 40101)
        │     ├─ 非 "Bearer xxx" 格式     → AppError::Unauthorized  (401 / 40101)
        │     ├─ JWT 签名无效             → AppError::Unauthorized  (401 / 40101)
        │     └─ JWT 已过期              → AppError::TokenExpired   (401 / 40102)
        │  3. 验证通过 → 注入 AuthContext { user_id: Uuid }
        │
        ▼ controller::get_me
        │  extract State(AppState) + Extension(RequestContext) + AuthContext
        │
        ▼ AuthService::get_me(user_id)
        │  1. user_repo.find_by_id(user_id)
        │     └─ None（含 deleted_at IS NOT NULL）→ AppError::NotFound("user")  (404 / 40400)
        │  2. 封禁检查：user.is_banned == true → AppError::Unauthorized          (401 / 40101)
        │  3. UserResponse::from(user)  ← 字段映射（见下表），排除敏感字段
        │
        ▼ 成功 → UserResponse { id, phone, nickname, avatar, coin_balance, vip_level, created_at }
        ▼ 响应 → ApiResponse<UserResponse> + request_id（来自 RequestContext）
```

### UserResponse DTO（`modules/auth/dto.rs`）

| 字段 | Rust 类型 | 说明 |
|------|-----------|------|
| `id` | `String` | 用户 UUID（`u.id.to_string()`）|
| `phone` | `String` | E.164 格式手机号 |
| `nickname` | `String` | 昵称 |
| `avatar` | `Option<String>` | 头像 URL，可为 `null` |
| `coin_balance` | `i64` | 金币余额 |
| `vip_level` | `i16` | VIP 等级 |
| `created_at` | `String` | 注册时间（`u.created_at.to_rfc3339()`，RFC 3339 格式）|

> **排除的敏感字段**：`password_hash`、`is_banned`、`deleted_at`、`is_new`（`is_new` 为登录专用字段，仅在 `LoginUserInfo` 中携带）

### request_id 透传机制（`common/middleware/jwt_auth.rs`）

JWT 鉴权失败时须在响应 body 中携带真实 `request_id`（protocol.md §1.3），通过 `FromRequestParts` 实现：

```rust
async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
    // request_context_middleware 已提前将 RequestContext 注入 extensions
    let request_id = parts
        .extensions
        .get::<RequestContext>()
        .map(|rc| rc.request_id().to_string())
        .unwrap_or_default();                          // 兜底：不应发生
    extract_auth_context(&parts.headers, &state.jwt_secret)
        .map_err(|e| e.into_rejection_with_id(&request_id))
}
```

> **修复背景**：T-00005 第三轮 Review（H-01）发现原 `into_rejection()` 硬编码 `"request_id": ""`；第四轮 Review 确认修复彻底，三种失败场景（无 token / 签名无效 / 过期）均返回真实 request_id，集成测试 `get_me_no_token_401_request_id_matches_header` 端到端验证通过。

---

## 六、Redis Key 设计（来自 protocol.md §6.2）

| Key 模式 | 类型 | TTL | 说明 |
|---------|------|-----|------|
| `sms:code:{phone}` | Hash | 300s | 字段：`code`（6位）、`attempts`（错误次数，上限 5）|
| `sms:cooldown:{phone}` | String | 60s | 值固定 `"1"`；冷却期内 Lua 拒绝重发 |
| `sms:daily:{phone}:{date}` | String (INCR) | 86400s | 日发送次数；上限 10 次；SMS 失败后不清除 |

### Lua 脚本说明

| 脚本常量 | 用途 | 原子性保证 |
|---------|------|-----------|
| `SAVE_CODE_LUA` | 检查冷却/日限 → 写入 code/cooldown/daily | 消除并发双发 TOCTOU |
| `VERIFY_CODE_LUA` | HGETALL → 校验 → 消费（DEL） | 消除同一 OTP 被双重消费 |

错误前缀统一使用 `VR:` 命名空间（`VR:COOLDOWN` / `VR:DAILY_LIMIT` / `VR:EXPIRED` / `VR:MAX_ATTEMPTS` / `VR:INVALID`），防止与 Redis 内部错误混淆。

---

## 七、错误码映射

| AppError 变体 | HTTP 状态码 | 业务错误码 | 说明 |
|--------------|-----------|----------|------|
| `InvalidPhoneNumber` | 400 | 40001 | 手机号非 E.164 格式 |
| `Unauthorized` | 401 | 40101 | 未登录或 token 无效 |
| `TokenExpired` | 401 | 40102 | JWT 已过期 |
| `InvalidVerificationCode` | 401 | 40103 | 验证码错误 |
| `VerificationCodeExpired` | 401 | 40104 | 验证码已过期或已使用 |
| `VerificationCodeMaxAttempts` | 401 | 40105 | 错误次数达上限（5次）|
| `VerificationCodeCooldown` | 429 | 42901 | 60 秒冷却期内重发 |
| `VerificationCodeDailyLimit` | 429 | 42902 | 当日发送超过 10 次 |
| `NotFound(_)` | 404 | 40400 | 资源不存在 |
| `SmsSendFailed(_)` | 500 | 50000 | SMS 发送失败（对外隐藏细节）|
| `DatabaseError(_)` | 500 | 50000 | 数据库错误（对外隐藏细节）|
| `RedisError(_)` | 500 | 50000 | Redis 错误（对外隐藏细节）|
| `Internal(_)` | 500 | 50000 | 内部错误（对外隐藏细节）|

> **安全说明**：所有 5xx 错误对外返回通用文本（`"internal server error"` 或 `"failed to send verification code"`），原始错误细节通过 `tracing::error!` 记录，不泄露给客户端（`safe_message()` 实现）。

---

## 八、SMS Provider 防腐层

| Provider | 文件 | 使用场景 |
|---------|------|---------|
| `TwilioSmsProvider` | `third_party/sms/twilio.rs` | 生产环境（`APP_ENV=prod`） |
| `MockSmsProvider` | `third_party/sms/mock.rs` | 开发/CI 环境，直接记录日志，不发真实短信 |
| `FailingSmsProvider` | `third_party/sms/mock.rs` | 测试异常路径（SMS 失败 + revoke_code 行为）|

生产与开发的 Provider 选择在 `main.rs` 启动时根据 `settings.app.environment == "prod"` 决策注入。

---

## 九、依赖注入图（AppState）

```
main.rs
  ├─ PgUserRepository::new(pool)    → Arc<dyn UserRepository>
  ├─ RedisCodeStore::new(redis_url).await  → Arc<dyn SmsCodeStore>
  ├─ TwilioSmsProvider / MockSmsProvider  → Arc<dyn SmsProvider>
  └─→ AppState::new(user_repo, code_store, sms, jwt_secret)
         └─→ AuthService::new(...)
               └─→ build_app(state) → Router
```

---

## 十、测试覆盖

| 测试位置 | 数量 | 覆盖范围 |
|---------|------|---------|
| `modules/auth/service.rs` | 19 | validate_phone、send_code（成功/冷却/日限/SMS失败）、login（正常/错误码/过期/封禁/复用OTP）、get_me（成功/not_found/banned）|
| `infrastructure/redis_store/mod.rs` | 4 | verify_and_consume 原子性、wrong→right 路径、daily_count 计数、revoke_code 契约 |
| `common/middleware/jwt_auth.rs` | 6 | JWT 鉴权 Extractor：合法 token / 无 header / 非 Bearer 格式 / 签名无效 / 过期 / iss 错误 |
| `common/error.rs` | 6 | HTTP 状态码与错误码映射 |
| `common/response.rs` | 1 | ApiResponse 序列化结构 |
| `bootstrap/mod.rs` | 2 | error/success 响应体中的 request_id 注入 |
| `lib.rs`（集成测试）| 2 | `get_me_no_token_401_request_id_matches_header`（端到端验证 request_id 与 X-Request-Id header 一致）、ping 健康检查 |
| **合计（server）** | **40** | 全部通过，`cargo clippy -- -D warnings` 零警告 |

---

## 十一、相关文档

- [Server 架构总索引](./index.md)
- [启动与目录结构](./structure.md)
- [能力状态与缺口盘点](./status.md)
- 接口契约：`doc/protocol.md` §二 §六
