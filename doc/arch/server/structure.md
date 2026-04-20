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

## 二、 当前启动流程

1. `ServerSettings::load()` 从 `.env`、`config/default.toml`、`config/{env}.toml` 与环境变量构建配置。
2. `init_tracing()` 按 `log.format` 选择 JSON 或普通文本日志。
3. `create_pool()` 初始化 SQLx PostgreSQL 连接池，并运行 `sqlx::migrate!` 自动迁移。
4. `RedisCodeStore::new(redis_url).await` 建立 Redis `MultiplexedConnection`（共享 TCP，Clone 复用）。
5. 按 `settings.app.environment` 选择 SMS Provider：`prod` 用 `TwilioSmsProvider`，其他用 `MockSmsProvider`。
6. `AppState::new(user_repo, code_store, sms, jwt_secret)` 组装依赖，构造 `AuthService`。
7. `build_app(state)` 注册 `/ping` + `auth_routes()`，注入 `request_context_middleware`。
8. Server 监听 `settings.server.bind_addr()`，并支持 `Ctrl+C` / `SIGTERM` 优雅退出。

## 三、 配置来源

| 来源 | 说明 |
| --- | --- |
| `.env.example` | 提供 `APP_ENV`、`DATABASE_URL`、日志配置等本地模板 |
| `config/default.toml` | 默认配置基线 |
| `config/dev.toml` / `test.toml` / `prod.toml` | 分环境覆盖 |
| 环境变量 | 支持 `APP__SERVER__HOST`、`APP__SERVER__PORT`、`APP__LOG__LEVEL`、`APP__LOG__FORMAT` 等覆盖 |

## 四、 当前测试面

- `src/bootstrap/mod.rs` 中的 `#[tokio::test]` 会直接对 `build_app()` 发起 HTTP 请求（集成测试）。
- `src/modules/auth/service.rs`、`src/infrastructure/redis_store/mod.rs`、`src/common/error.rs` 包含单元测试。
- 测试覆盖点为：
  - `/ping` 返回 `200 OK`，响应头包含 `x-request-id`，响应体中的 `request_id` 与响应头一致
  - `POST /api/v1/auth/verification-codes` 错误/成功响应体中 `request_id` 正确注入（H-01 集成验证）
  - `AuthService::send_code` 正向成功、冷却期拒绝、日限拒绝、SMS 失败撤销 cooldown
  - `AuthService::login` 正确码登录、错误码、过期码、封禁用户、OTP 不可复用
  - `AuthService::get_me` 正常返回、用户不存在、封禁用户
  - Redis `verify_and_consume` 原子性（同一 OTP 仅可消费一次）
  - Redis `revoke_code` 清除 code + cooldown、保留 daily count
  - `AppError` HTTP 状态码与业务错误码映射
- **当前通过测试数：196 个，`cargo clippy -- -D warnings` 零警告**

## 五、 结论
Server 端 Auth 业务域（T-00002 ~ T-00005）、Room 业务域（T-00006 ~ T-00010、T-00012 ~ T-00016）、WebSocket 网关（T-00011、T-00011B、T-00011C）已全部落地，具备完整的鉴权、房间 CRUD、实时通信与在线统计能力。下一步应优先展开支付业务域。
