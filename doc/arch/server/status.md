# Server 能力状态与缺口盘点

## 一、 已实现能力

### 1. 健康检查
- 已提供 `GET /ping`。
- 返回体结构为 `{ "status": "ok", "request_id": "..." }`。
- 可以复用作存活探针与链路排障的最小入口。

### 2. 请求上下文
- 中间件会优先透传请求头中的 `x-request-id`。
- 若上游未提供，则自动生成 UUID。
- `request_id` 会同时进入响应头与 tracing span。
- 所有 handler 错误路径通过 `err_response(e, rc.request_id())` 将 `request_id` 注入响应体 JSON。

### 3. 配置与日志骨架
- 已支持 `.env` + `config/*.toml` + 环境变量覆盖的组合加载。
- 日志格式支持 `json` 与普通文本两种模式。
- 启动日志会携带 `service_name`、`environment`、`host`、`port` 等字段。

### 4. 数据库（SQLx 0.8 + PostgreSQL）
- `create_pool()` 初始化连接池，`max_connections` 与 `connect_timeout_secs` 可配置。
- 启动时调用 `voice_room_shared::migrate::run_migrations_with_table(pool, &sqlx::migrate!("./migrations"), "_sqlx_app_migrations")`，按 T-0000M 决议使用自定义登记表，避免与 AdminServer（`_sqlx_admin_migrations`）共库时互相覆盖。

### 5. Redis
- `RedisCodeStore::new(redis_url).await` 建立 `MultiplexedConnection`，内部共享同一 TCP 连接，每次操作 `.clone()` 复用，无并发 `&mut` 竞争。
- 关键操作（`save_code` / `verify_and_consume`）使用 Lua 脚本原子化，错误前缀统一 `VR:` 命名空间。

### 6. SMS 防腐层（SmsProvider）
- `SmsProvider` trait 隔离 Twilio / 其他平台，生产用 `TwilioSmsProvider`，开发/CI 用 `MockSmsProvider`（no-op，记录日志）。
- 按 `settings.app.environment == "prod"` 在 `main.rs` 启动时注入。

### 7. Auth 模块（T-00002 ～ T-00005）
- `POST /api/v1/auth/verification-codes`：发送短信验证码，含冷却/日限/原子写入/SMS 失败撤销。
- `POST /api/v1/auth/login`：验证码登录，自动注册新用户，签发 JWT（30 天有效期）。
- JWT 鉴权中间件：无/非法/过期 token 返回 401，合法 token 注入 `user_id`。
- `GET /api/v1/users/me`：需 JWT 鉴权，返回完整用户信息（不含敏感字段）。

### 8. 统一错误响应
- `AppError` 枚举覆盖 400 / 401 / 404 / 429 / 500 全部业务场景。
- `safe_message()`：5xx 对外返回通用文本，原始细节通过 `tracing::error!` 记录，不泄露给客户端。

## 二、 未实现能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 房间业务域 — 数据层（`rooms` 表） | 🟢 已完成 | T-00006：`002_create_rooms.sql` DDL + `RoomModel` struct，详见 [database.md](./database.md) |
| 房间业务域 — HTTP 接口 | 🟢 已完成 | T-00007（创建）、T-00008（列表）、T-00009（详情）、T-00010（关闭）全部落地 |
| WebSocket 网关 | 🟢 已完成 | T-00011（连接管理）、T-00011B（Redis 事件订阅）、T-00011C（在线统计），详见 [websocket.md](./websocket.md) |
| 房间运行时（WS 信令） | 🟢 已完成 | T-00012（进入）、T-00013（离开）、T-00014（上麦）、T-00015（下麦）、T-00016（文本消息），详见 [room_runtime.md](./room_runtime.md) |
| Admin 房间管理接口 | 🟢 已完成 | T-10004（列表）、T-10005（详情）、T-10006（强制关闭） |
| 礼物 HTTP 端点 | 🟢 已完成 | T-00044（`POST /api/v1/gifts/send`，复用 T-00020 核心逻辑，Idempotency-Key 幂等，错误码与 WS 对齐 40004/40290/40402/40403） |
| 支付业务域 | 🔴 未开始 | coin_balance 已预留字段，业务逻辑未开始 |

## 三、 文档维护约束

- 当新增 HTTP/WS 契约时，应同步补齐 `doc/protocol/` 目录下对应子文件。
- 当 `modules/` 下新增业务域时，需在 `doc/arch/server/` 下为其创建对应子文档，并更新 `index.md` 索引。
- 当数据库与事务接入后，需要在本目录同步补充"事务边界"和"幂等策略"说明。
- **当前通过测试数**：App Server 196 个 + Admin Server 74 个，`cargo clippy -- -D warnings` 零警告。