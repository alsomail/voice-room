# Server 启动、配置与目录结构

## 一、 目录与文件现状

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/shared/` | Workspace 共享 crate：JWT 工具、密码哈希、错误码、数据模型、新类型 | 🟢 已落地 |
| `app/server/src/main.rs` | 读取配置、初始化 tracing、构建连接池 / Redis / SMS Provider，启动 Axum Server | 🟢 已落地 |
| `app/server/src/bootstrap/mod.rs` | 注册 `/ping` 路由、挂载 `auth_routes()`、注入请求上下文中间件；定义 `AppState` | 🟢 已落地 |
| `app/server/src/common/request_context.rs` | 定义 `RequestContext`，封装 `request_id` | 🟢 已落地 |
| `app/server/src/common/error.rs` | `AppError` 枚举、`err_response()`、`safe_message()`（防 5xx 信息泄露） | 🟢 已落地 |
| `app/server/src/common/response.rs` | `ApiResponse<T>` 统一成功响应结构 | 🟢 已落地 |
| `app/server/src/common/auth/` | `AuthContext` Axum Extractor（JWT 鉴权，T-00004） | 🟢 已落地 |
| `app/server/src/infrastructure/config.rs` | 负责 `.env`、`config/*.toml` 与环境变量覆盖的配置加载 | 🟢 已落地 |
| `app/server/src/infrastructure/logging.rs` | 负责 tracing 初始化、`x-request-id` 透传与请求日志 span | 🟢 已落地 |
| `app/server/src/infrastructure/database/` | SQLx 连接池（`create_pool`）与 migration 运行入口 | 🟢 已落地 |
| `app/server/src/infrastructure/redis_store/mod.rs` | `SmsCodeStore` trait + `RedisCodeStore`（`MultiplexedConnection` 复用）+ `FakeCodeStore`（测试） | 🟢 已落地 |
| `app/server/src/infrastructure/third_party/sms/mod.rs` | `SmsProvider` trait + re-export | 🟢 已落地 |
| `app/server/src/infrastructure/third_party/sms/twilio.rs` | `TwilioSmsProvider`（生产 HTTP 调用） | 🟢 已落地 |
| `app/server/src/infrastructure/third_party/sms/mock.rs` | `MockSmsProvider`（开发/CI no-op）、`FailingSmsProvider`（异常路径测试） | 🟢 已落地 |
| `app/server/src/modules/auth/routes.rs` | `auth_routes()` 注册三条 Auth 路由 | 🟢 已落地 |
| `app/server/src/modules/auth/controller.rs` | `send_code` / `login` / `get_me` handler，统一调用 `err_response()` | 🟢 已落地 |
| `app/server/src/modules/auth/service.rs` | `AuthService`（send_code / login / get_me）+ 内联单元测试 | 🟢 已落地 |
| `app/server/src/modules/auth/dto.rs` | `SendCodeRequest/Response`、`LoginRequest/Response`、`UserResponse` | 🟢 已落地 |
| `app/server/src/modules/auth/repository.rs` | `UserRepository` trait + `PgUserRepository`（SQLx）+ `FakeUserRepository`（测试） | 🟢 已落地 |
| `app/server/src/modules/room/routes.rs` | `room_routes()` 注册房间 CRUD 路由（T-00007~T-00010） | 🟢 已落地 |
| `app/server/src/modules/room/controller.rs` | 创建/列表/详情/关闭房间 handler | 🟢 已落地 |
| `app/server/src/modules/room/service.rs` | `RoomService`（创建/列表/详情/关闭）+ 单元测试 | 🟢 已落地 |
| `app/server/src/modules/room/repository.rs` | `RoomRepository` trait + `PgRoomRepository` + `FakeRoomRepository` | 🟢 已落地 |
| `app/server/src/ws/handler.rs` | WS 握手 + JWT 鉴权（T-00011） | 🟢 已落地 |
| `app/server/src/ws/registry.rs` | `ConnectionRegistry`（DashMap 连接注册表） | 🟢 已落地 |
| `app/server/src/ws/heartbeat.rs` | 心跳检测 task（10s 扫描，30s 超时） | 🟢 已落地 |
| `app/server/src/ws/connection.rs` | 单连接生命周期 + 信令路由 | 🟢 已落地 |
| `app/server/src/events/admin_event.rs` | `AdminEvent` enum（BanUser/CloseRoom/BroadcastNotice，T-00011B） | 🟢 已落地 |
| `app/server/src/events/handler.rs` | 三路事件处理（ban/close/broadcast） | 🟢 已落地 |
| `app/server/src/events/subscriber.rs` | Redis Pub/Sub 订阅者 + 自动重连 | 🟢 已落地 |
| `app/server/src/stats/service.rs` | `StatsPort` trait + `StatsService`（Redis HLL/Set，T-00011C） | 🟢 已落地 |
| `app/server/src/stats/snapshot_task.rs` | 定时快照 task（60s 间隔 + 优雅停机） | 🟢 已落地 |
| `app/server/src/room/manager.rs` | `RoomManager`（DashMap 房间运行时状态管理，T-00012） | 🟢 已落地 |
| `app/server/src/room/state.rs` | `RoomState`（成员表 + 麦位 + banned_mics + muted_users + msg_id 去重） | 🟢 已落地 |
| `app/server/src/room/handler.rs` | 进/退房 + 上/下麦 + 文本消息 handler（T-00012~T-00016） | 🟢 已落地 |
| `app/server/src/room/filter.rs` | 敏感词过滤（T-00016） | 🟢 已落地 |
| `app/server/src/lib.rs` | 暴露模块并包含 `/ping` 及 Auth/Room 路由集成测试 | 🟢 已落地 |

## 二、 当前启动流程（含 T-00040 config 多 profile 体系）

### 启动阶段详解

1. **配置加载**（`ServerSettings::load()`，T-00040 新增）：
   - 加载链：`.env` → 解析 `APP_PROFILE`（优先级 `APP_PROFILE > APP_ENV > APP__ENVIRONMENT > "dev"` 默认）→ 白名单校验 `{dev,test,staging,prod}` → 默认值 → `default.toml` → `{profile}.toml` → ENV 覆盖
   - 敏感字段 fail-fast：`DATABASE_URL` / `JWT_SECRET` 必填（缺失/占位符退出码 78），`REDIS_URL` 仅 dev 允许回退 `redis://127.0.0.1:6379`
   - 新支持 ENV 覆盖：`APP__JWT__EXPIRE_SECS`、`APP__DATABASE__MAX_CONNECTIONS`、`APP__DATABASE__CONNECT_TIMEOUT_SECS`
   - 启动摘要日志：含 profile、host、port、DB/Redis 凭据脱敏、JWT 密钥长度（永不打印明文）

2. `init_tracing()` 按 `log.format` 选择 JSON 或普通文本日志。

3. `create_pool()` 初始化 SQLx PostgreSQL 连接池；通过 `voice_room_shared::migrate::run_migrations_with_table` 以自定义登记表 `_sqlx_app_migrations` 运行迁移，避免与 AdminServer 的 `_sqlx_admin_migrations` 互掐（详见 [T-0000M](../../tds/infra/T-0000M.md) / [ADR-0001](../../adr/ADR-0001-migration-table-isolation.md)）。

4. `RedisCodeStore::new(redis_url).await` 建立 Redis `MultiplexedConnection`（共享 TCP，Clone 复用）。

5. 按 `settings.app.environment` 选择 SMS Provider：`prod` 用 `TwilioSmsProvider`，其他用 `MockSmsProvider`。

6. `AppState::new(user_repo, code_store, sms, jwt_secret)` 组装依赖，构造 `AuthService`。

7. `build_app(state)` 注册 `/ping` + `auth_routes()`，注入 `request_context_middleware`。

8. Server 监听 `settings.server.bind_addr()`，并支持 `Ctrl+C` / `SIGTERM` 优雅退出。

### T-00040 fail-fast 行为

当启动失败时（缺 `DATABASE_URL`、缺 `JWT_SECRET`、无效 profile 等），`main.rs` 捕获 anyhow Error 并输出：
```
CONFIG ERROR: <错误正文>
```
然后 `std::process::exit(78)`（EX_CONFIG，便于 preflight 脚本定位）。

## 三、 配置加载与多 profile 体系（T-00040）

### 3.1 配置来源与加载链

| 来源 | 说明 | 涵盖 T-00040 |
| --- | --- | --- |
| `.env.example` | 提供 `APP_PROFILE=dev`（主入口）、`DATABASE_URL`、`REDIS_URL`、`JWT_SECRET`、日志配置等本地模板 | ✅ |
| `config/default.toml` | 默认配置基线，包含 `[app]` `[server]` `[database]` `[redis]` `[jwt]` `[log]` 6 个章节，所有敏感字段由 ENV 注入 | ✅ |
| `config/dev.toml` | Dev profile 差异：`log.level=debug` | ✅ |
| `config/test.toml` | Test profile 差异：`server.port=4000`、`jwt.expire_secs=3600` | ✅ |
| `config/staging.toml` | **Staging profile 新增**：`environment="staging"`、`log.format=json`、`log.level=info` | ✅ 新增 |
| `config/prod.toml` | Prod profile 差异：`server.port=3000`、`log.format=json`、`database.max_connections=50`、`database.connect_timeout_secs=10` | ✅ |
| 环境变量 | 支持 `APP_PROFILE` / `APP_ENV`（向后兼容）、`APP__SERVER__*`、`APP__LOG__*`、`APP__JWT__*`、`APP__DATABASE__*` 全系列覆盖，优先级最高 | ✅ |

### 3.2 Profile 白名单与默认值

| Profile | 环境变量值 | `app.environment` | `log.level` | `server.port` | 特殊行为 |
| --- | --- | --- | --- | --- | --- |
| `dev` | `APP_PROFILE=dev` | `"dev"` | `debug` | `3000` | REDIS_URL 缺失时回退 `127.0.0.1:6379`（仅此 profile），其余必填 |
| `test` | `APP_PROFILE=test` | `"test"` | `info` | `4000` | 端口独立避免与 AdminServer 3001 冲突 |
| `staging` | `APP_PROFILE=staging` | `"staging"` | `info` | `3000` | 全部敏感字段严格必填 |
| `prod` | `APP_PROFILE=prod` | `"prod"` | `info` | `3000` | 全部敏感字段严格必填，log 强制 JSON |

> 若 `APP_PROFILE` 不在白名单，启动立即失败：`CONFIG ERROR: invalid APP_PROFILE='xxx'; expected one of [dev,test,staging,prod]` → 退出码 78

### 3.3 必填字段与 fail-fast 契约

| 必填字段 | 校验点 | 缺失/空时 | 退出码 | 错误消息前缀 |
| --- | --- | --- | --- | --- |
| `DATABASE_URL` | **所有 profile** | 启动失败 | 78 | `CONFIG ERROR: DATABASE_URL must be set` |
| `JWT_SECRET` | **所有 profile** | 启动失败；占位符 `change-me-in-production` 亦拒绝 | 78 | `CONFIG ERROR: JWT_SECRET must be set` 或 `still equals the placeholder` |
| `REDIS_URL` | dev 允许缺失（日志 WARN + fallback）；**其他 profile 必填** | dev 回退内置值；非 dev 启动失败 | 78 (非dev) | `CONFIG ERROR: REDIS_URL must be set for non-dev profile` |



## 四、 配置文件字段冻结表（§2.3 完整骨架）

### 默认配置 `default.toml`（profile-agnostic 基线）

```toml
[app]
name = "voice-room-server"
environment = "dev"               # 会被 {profile}.toml / APP_PROFILE 覆盖

[server]
host = "0.0.0.0"
port = 3000

[database]
# DSN 完全由 DATABASE_URL 注入，禁止在 toml 写入明文凭据
max_connections = 10
connect_timeout_secs = 5

[redis]
# URL 完全由 REDIS_URL 注入；预留章节用于 future override

[jwt]
# 密钥由 JWT_SECRET 注入
expire_secs = 86400

[log]
level = "info"
format = "json"
service_name = "voice-room-server"
```

### 分 Profile 差异表

| 配置项 | `default.toml` | `dev.toml` | `test.toml` | `staging.toml` | `prod.toml` |
|---|---|---|---|---|---|
| `app.environment` | `"dev"` | — | `"test"` | `"staging"` | `"prod"` |
| `log.level` | `"info"` | `"debug"` | — | — | — |
| `log.format` | `"json"` | — | — | — | — |
| `server.port` | `3000` | — | `4000` | — | `3000` |
| `database.max_connections` | `10` | — | — | — | `50` |
| `database.connect_timeout_secs` | `5` | — | — | — | `10` |
| `jwt.expire_secs` | `86400` | — | `3600` | — | — |

**说明**：
- 空白 `—` 表示沿用 `default.toml` 值
- `dev.toml` 最轻量（仅 `log.level=debug`）
- `test.toml` 独立端口 4000 避免冲突，JWT token 短期 3600s 便于测试
- `staging.toml` 生产级配置（环境标签 + JSON 日志）
- `prod.toml` 最严格（大连接池 50、长超时 10s、JSON 日志）

## 五、 当前测试面

- `src/bootstrap/mod.rs` 中的 `#[tokio::test]` 会直接对 `build_app()` 发起 HTTP 请求（集成测试）。
- `src/modules/auth/service.rs`、`src/infrastructure/redis_store/mod.rs`、`src/common/error.rs` 包含单元测试。
- **T-00040 新增** `tests/server_settings_load_test.rs`：集成测试 3 cases（I1/I2/I3），覆盖 staging profile 加载、ENV override、必填字段校验。
- **T-00040 新增** `src/infrastructure/config.rs` 单元测试 28 cases（U1.1~U1.5 profile 解析 / U2.1~U2.5 加载链优先级 / U3.1~U3.5 敏感字段 / U4.1~U4.4 ENV override / U5.1~U5.3 日志脱敏）。
- 测试覆盖点为：
  - `/ping` 返回 `200 OK`，响应头包含 `x-request-id`，响应体中的 `request_id` 与响应头一致
  - `POST /api/v1/auth/verification-codes` 错误/成功响应体中 `request_id` 正确注入（H-01 集成验证）
  - `AuthService::send_code` 正向成功、冷却期拒绝、日限拒绝、SMS 失败撤销 cooldown
  - `AuthService::login` 正确码登录、错误码、过期码、封禁用户、OTP 不可复用
  - `AuthService::get_me` 正常返回、用户不存在、封禁用户
  - Redis `verify_and_consume` 原子性（同一 OTP 仅可消费一次）
  - Redis `revoke_code` 清除 code + cooldown、保留 daily count
  - `AppError` HTTP 状态码与业务错误码映射
- **当前通过测试数：636 个，`cargo clippy -- -D warnings` 零警告**（T-00040 增加了 31 个新增单元测试 + 3 个集成测试）

## 六、 结论

Server 端 Auth 业务域（T-00002 ~ T-00005）、Room 业务域（T-00006 ~ T-00010、T-00012 ~ T-00016）、WebSocket 网关（T-00011、T-00011B、T-00011C）已全部落地，具备完整的鉴权、房间 CRUD、实时通信与在线统计能力。**T-00040 补全服务端配置体系**，引入 APP_PROFILE 多环境（dev/test/staging/prod）分层与 fail-fast 机制，满足模块 9 E2E 测试基建需求。下一步应优先展开支付业务域。
