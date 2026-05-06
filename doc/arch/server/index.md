<!--
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Server 架构的总路由，严禁在此文件内编写具体业务逻辑或冗长代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【二、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【三、当前能力全景与状态】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Server 端架构总索引与状态盘点

## 一、 架构概述
Server 端基于 Rust + Axum 构建。启动骨架（配置、日志、健康检查）已完成；Auth 业务域（短信验证码、手机号登录、JWT 鉴权、用户信息）已全部落地并通过 Review；数据库（SQLx 0.8 + PostgreSQL）与 Redis 已接入运行链路；Room 业务域数据层（`rooms` 表 DDL + `RoomModel` struct，T-00006）已完成；**创建房间接口**（`POST /api/v1/rooms`，T-00007）已落地（含 JWT 鉴权、参数校验、bcrypt 密码哈希、唯一 active 房间约束，60 个单元测试全通过）；**房间列表接口**（`GET /api/v1/rooms`，T-00008）已落地（分页热度排序、无鉴权，78 个测试全通过）；**房间详情接口**（`GET /api/v1/rooms/:id`，T-00009）已落地（公开无鉴权、UUID 路径参数校验、返回房主信息与麦位列表 MVP 为空）；**关闭房间接口**（`DELETE /api/v1/rooms/:id`，T-00010）已落地（JWT 鉴权、仅房主可操作、active→closed 状态变更、409 冲突检测，MVP 阶段暂不广播 WebSocket 事件）；**Admin 房间列表接口**（`GET /api/v1/admin/rooms`，T-10004）已落地（Admin JWT 鉴权、finance 角色 403 拦截、分页 + 状态过滤 + 关键词搜索、可见 closed 房间）；**Admin 房间详情接口**（`GET /api/v1/admin/rooms/:id`，T-10005）已落地（Admin JWT 鉴权、finance 角色 403 拦截、可见 active/closed 状态房间、UUID 路径参数校验、响应含 status 与 updated_at 字段）；**Admin 强制关闭房间接口**（`DELETE /api/v1/admin/rooms/:id`，T-10006）已落地（Admin JWT 鉴权、仅 super_admin/operator 角色有 RoomForceClose 权限、无 owner 检查、active→closed 状态变更、404/409 冲突检测，MVP 阶段暂不广播 WebSocket 事件）；**WebSocket 连接管理**（`GET /ws?token=<JWT>`，T-00011）已落地（`src/ws/` 模块，JWT 握手鉴权、`ConnectionRegistry` DashMap 无锁并发注册、心跳检测 task 支持优雅停机、tokio::select! 双向读写、`connection_id` 解耦 `user_id` 防多连接注销竞争，13 个测试全通过，全量 122 passed）；**Redis 事件订阅**（`admin:events` 频道，T-00011B）已落地（`src/events/` 模块，`AdminEvent` serde internally-tagged 反序列化、`handle_admin_event` 三路事件处理（ban_user/close_room/broadcast_notice）、`tokio::spawn` 隔离单事件失败、Redis Pub/Sub 自动重连 task + shutdown 优雅停机、`ConnectionRegistry` 扩展 `room_id: Option<Uuid>` + `get_by_user_id()` + `get_connections_in_room()`，11 个新增测试，全量 133 passed, 0 failed）；**在线统计上报**（T-00011C）已落地（`src/stats/` 模块，`StatsPort` trait + `StatsService` Redis 实现（HLL/Set/原子 pipeline）+ `FakeStatsService` 测试替身、`snapshot_task` 每 60s 定时快照支持 shutdown watch channel 优雅停机、WS handle_socket 入口/退出调用 user_online/user_offline 失败 .ok() 不阻断主流程，10 个新增测试，全量 143 passed, 0 failed）；**进入房间逻辑**（T-00012）已落地（`src/room/` 模块，`RoomManager` DashMap 全局内存状态、`RoomState` 成员表与麦位、`handle_join_room` WS 信令处理、`registry.set_room_id` 关联连接与房间、DB 校验→内存更新→广播 `UserJoined` 有序三步，11 个新增测试，全量 154 passed, 0 failed）；**离开房间逻辑**（T-00013）已落地（`src/room/` 模块扩展，`room/state.rs` 新增 `remove_from_mic_slots(user_id)` 自动下麦、`room/handler.rs` 新增 `do_leave_room` 主被动路径复用逻辑与 `handle_leave_room` WS 信令入口、`ws/registry.rs` 新增 `get_room_id` / `clear_room_id` 方法、`ws/connection.rs` LeaveRoom 信令路由与断线自动触发退房，10 个新增测试，全量 164 passed, 0 failed）；**上麦接口**（T-00014）已落地（`room/state.rs` 新增 `TakeMicError` enum、`banned_mics: DashSet<Uuid>`、`take_mic_slot` 同步原子方法（写锁不跨 await，并发抢麦只有一个成功）、`room/handler.rs` 新增 `TakeMicDeps` 与 `handle_take_mic` 7 步流程（payload 解析→房间校验→禁麦检查→原子占位→广播 `MicTaken`→响应）、`ws/connection.rs` TakeMic 信令路由分支，9 个新增测试，全量 173 passed, 0 failed）；**下麦接口**（T-00015）已落地（`room/state.rs` 新增 `leave_mic_slot(user_id) -> Option<usize>` 原子方法（写锁不跨 await）、`room/handler.rs` 新增 `LeaveMicDeps`、`handle_leave_mic` 5 步流程、`broadcast_mic_left` 普通 fn、`do_leave_room` 先暂存麦位再广播保证顺序、`ws/connection.rs` LeaveMic 信令路由分支，全量 182 passed, 0 failed）；**文本消息广播**（T-00016）已落地（`room/filter.rs` 新建 `filter_content` 敏感词净化、`room/state.rs` 新增 `muted_users: DashSet<Uuid>` 与 `processed_msg_ids: DashSet<String>`、`room/handler.rs` 新增 `SendMessageDeps` 与 `handle_send_message` 8 步流程（内容校验→长度限制→房间校验→禁言检查→幂等去重→净化→广播→响应）、`ws/connection.rs` SendMessage 信令路由分支，全量 196 passed, 0 failed）；**礼物 HTTP 端点 POST /api/v1/gifts/send**（T-00044）已落地（复用 T-00020 GiftSendService、Idempotency-Key 幂等、错误码与 WS 对齐 40004/40290/40402/40403、广播+榜单同步执行、9 HTTP+12 WS 测试全绿）；支付业务域仍未展开。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：请先通过以下子文档确认“当前已实现的骨架”和“尚未落地的业务边界”，再决定是否继续扩展。

### 实际目录：
- 🧱 [启动、配置与目录结构](./structure.md) - `main.rs`、`bootstrap`、`config`、`logging`、数据库 / Redis 初始化与测试入口现状。
- 📊 [能力状态与缺口盘点](./status.md) - 现有可用能力、未落地模块与下一步约束。
- 🔐 [Auth 模块架构](./auth.md) - 短信验证码（T-00002）、手机号登录（T-00003）、JWT 中间件（T-00004）、获取用户信息（T-00005）的路由、服务、Redis Key 设计与错误码映射。
- 🗄️ [数据库 Schema 设计](./database.md) - 各业务表 DDL 说明、字段约束、索引策略与 Rust 模型映射（含 `rooms` 表 T-00006、房间治理扩字段 + `room_kick_records` + `room_mute_records` T-00024、`chat_messages` 持久化表 T-00043）。
- 🔌 [WebSocket 模块架构](./websocket.md) - WS 握手鉴权（T-00011）、`ConnectionRegistry`、心跳检测、单连接生命周期与 `connection_id` 解耦设计。
- 🏠 [房间运行时模块](./room_runtime.md) - `src/room/` 模块说明：`RoomManager`（DashMap 全局状态）、`RoomState`（成员表 + 麦位 + `banned_mics` + `muted_users` + `processed_msg_ids`）、`handle_join_room` WS 信令处理（T-00012）、`do_leave_room` / `handle_leave_room` 离开房间逻辑（T-00013）、`take_mic_slot` / `handle_take_mic` 上麦逻辑（T-00014）、`leave_mic_slot` / `handle_leave_mic` 下麦逻辑（T-00015）、`filter_content` 敏感词净化 / `handle_send_message` 文本消息 WS 广播（T-00016）；`src/modules/chat/` 聊天历史 REST API 与 DB 持久化（T-00043）。
- 💰 [Wallet 模块架构](./wallet.md) - 余额查询 API（T-00018）、流水分页查询、`WalletService.apply_delta` 原子事务支持、`BalanceBroadcaster` Redis PubSub 跨进程推送、WS `BalanceUpdated` 信令设计。
- 🎁 [礼物模块架构](./gift.md) - 礼物配置表与列表 API（T-00019）、国际化支持（Accept-Language）、缓存策略（60s 进程内存）、`GiftModel` 数据模型；SendGift 事务编排（T-00020）、6 步强事务流程、msg_id 幂等、并发超扣防护、房间广播与发送者推送；**HTTP 礼物端点 POST /api/v1/gifts/send**（T-00044）、Idempotency-Key 幂等、错误码与 WS 对齐、广播+榜单同步执行、9 HTTP+12 WS 测试全绿。
- 📊 [Analytics 模块架构](./analytics.md) - 事件表 Schema + 分区设计（T-00022）、HTTP `POST /api/v1/events/batch` 批量接收接口、PartitionScheduler 定时分区创建 + 补偿逻辑；WebSocket `ReportEvent` 信令（T-00023）与 EventWriter 共享写入层、JWT user_id 覆盖逻辑、properties 8KB 截断机制。
- 🏠 [Room HTTP API 架构](./room.md) - `POST /api/v1/rooms` 扩展字段（`cover_url`/`category`/`announcement`/`password`）与白名单校验（T-00025）；`PATCH /api/v1/rooms/:id` 房主更新接口；`RoomInfoUpdated` WS 广播格式（含 `has_password` 布尔）；`validator.rs` 四个验证函数设计；遗留 MEDIUM 项（`BroadcastEnvelope` 缺 `msg_id`）。

## 🔌 协议入口索引 (Protocol Entry Index)

> **铁律**：每个跨端 Task 的 DoD 阶段必须把 TDS「协议路径绑定表」中**本端涉及的行**反向写入此表。本表是 server 端**所有**对外协议入口的汇总，供 global-review、新人 onboarding 和重构变更影响面分析使用。

### 🔌 Schema 索引
- [Protocol Schemas](../../protocol/schemas/) — WS/HTTP/Pub/Sub 三协议层机器可读 Schema（T-00100 落锚）
  - `schemas/ws/` — 34 个 WebSocket 信令 Schema（含 Ping/Pong/JoinRoom/SendMessage 等）
  - `schemas/http/` — HTTP DTO Schema（含 RoomDetail.mic_slots 强类型）
  - `schemas/pubsub/` — Redis admin:events Schema（BanUser/UnbanUser/CloseRoom/BroadcastNotice）

| 协议类型 | 入口 / 信令 | 实现文件:函数 | protocol/ 锚点 | 关联 Task | 客户端实调用方 |
|----------|------------|---------------|---------------|-----------|----------------|
| WS C→S | `SendMessage` ⭐ | `app/server/src/room/handler/chat.rs::handle_send_message` | [websocket_signals.md §6.8.1](../../protocol/websocket_signals.md) | T-00047 | `app/android/app/src/main/java/com/voice/room/android/feature/room/RoomViewModel.kt::sendMessage` |
| WS S→Room 广播 | `RoomMessage` | `app/server/src/ws/broadcaster.rs::broadcast_to_room` | [websocket_signals.md §6.8.2](../../protocol/websocket_signals.md) | T-00047 | Android `RoomViewModel` 接收 `type == "RoomMessage"` 后分发到 Chat UI |
| HTTP REST | `POST /api/v1/chat-messages` | `app/server/src/modules/chat/controller.rs::send_chat_message_handler` | [room_api.md §3.6.1](../../protocol/room_api.md) | T-00047 | 当前无 C 端客户端调用；运营 / 后端兜底备路径 |
| 集成测试 | `chat_dual_path_equivalence` | T-00048 DUAL-1/2/3 双路径等价回归测试 | - | T-00048 | `app/server/tests/chat_dual_path_equivalence.rs` |

### Redis Pub/Sub（admin:events 消费方）
| # | 事件类型 | 订阅处理入口 | 发布方 | Schema |
|---|---------|------------|--------|--------|
| 1 | `admin:events :: BanUser` | `events/handler.rs::handle_admin_event::BanUser` | adminServer/user/service.rs | [BanUser.schema.json](../../protocol/schemas/pubsub/BanUser.schema.json) |
| 2 | `admin:events :: UnbanUser` | `events/handler.rs::handle_admin_event::UnbanUser` | adminServer/user/service.rs | [UnbanUser.schema.json](../../protocol/schemas/pubsub/UnbanUser.schema.json) |
| 3 | `admin:events :: CloseRoom` | `events/handler.rs::handle_admin_event::CloseRoom` | adminServer/room/service.rs | [CloseRoom.schema.json](../../protocol/schemas/pubsub/CloseRoom.schema.json) |
| 4 | `admin:events :: BroadcastNotice` | `events/handler.rs::handle_admin_event::BroadcastNotice` | adminServer/event/notice_service.rs | [BroadcastNotice.schema.json](../../protocol/schemas/pubsub/BroadcastNotice.schema.json) |

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🟢 Server 启动装配、优雅停机与 Axum 路由注册
- 🟢 `GET /ping` 健康检查、JSON 响应与 `x-request-id`
- 🟢 `GET /health` 统一轻量探活端点（T-0000N）：200 OK + `{status:"ok", service:"app-server", version:"x.x.x"}`，零鉴权、零依赖，与 `/ping` 同层挂载，供 wait-on / preflight / 监控探针使用
- 🟢 tracing 初始化、请求级 span 与访问日志字段注入
- 🟢 `app/shared` crate 集成（JWT encode/decode + iss 校验、bcrypt 密码工具、公共错误码）
- 🟢 配置分层读取（`.env` + `config/*.toml` + 环境变量覆盖）
- 🟢 数据库连接池（SQLx 0.8 + PostgreSQL）与自动 migration（`sqlx::migrate!`）；**双服务共库迁移表隔离**（T-0000M）：AppServer 使用自定义表 `_sqlx_app_migrations` 由 `voice_room_shared::migrate::run_migrations_with_table` helper 接管
- 🟢 Redis 连接（`MultiplexedConnection` 缓存复用）
- 🟢 **Auth 模块**：`POST /api/v1/auth/verification-codes`（T-00002）、`POST /api/v1/auth/login`（T-00003）、JWT 鉴权中间件（T-00004）、`GET /api/v1/users/me`（T-00005）
- 🟢 SMS 防腐层（`SmsProvider` trait）：生产用 Twilio，开发/CI 用 Mock
- 🟢 统一错误响应结构（含 `request_id`、`safe_message` 防信息泄露）
- 🟢 **数据层 — rooms 表**（T-00006）：`002_create_rooms.sql` DDL（6 个 CHECK 约束、3 个索引含软删除偏滤）+ `RoomModel` struct（29 个单元测试全通过）
- 🟢 **E-10 Schema 基座 — rooms 扩字段 + 治理审计表**（T-00024）：`008_room_governance.sql` 幂等迁移（`cover_url`/`category`/`announcement`/`admin_user_id` 扩字段 + `chk_room_category` 6类枚举约束）；新建 `room_kick_records`（踢人审计，2 索引）+ `room_mute_records`（禁言/禁麦审计，`type CHECK IN('mic','chat')` + 2 索引）；`RoomKickRecord`/`RoomMuteRecord`/`MuteType` 模型；23 个测试全通过
- 🟢 **创建房间 API 升级**（T-00025）：`POST /api/v1/rooms` 新增 `cover_url`/`category`/`announcement`/`password` 字段；白名单封面校验（`validator.rs` `validate_cover_url`）；密码 bcrypt hash 写库明文不落地；新建 `PATCH /api/v1/rooms/:id`（仅房主，更新 title/announcement/category）；变更后广播 `RoomInfoUpdated` WS 信令（含 `has_password` 布尔）；369 个测试全通过 ✅；遗留 MEDIUM：`BroadcastEnvelope` 缺 `msg_id`（不阻塞上线）
- 🟢 **房间创建接口**（T-00007）：`POST /api/v1/rooms`（JWT 鉴权、标题校验、唯一 active 房间约束、bcrypt 密码哈希、HTTP 201 响应）；`003_add_unique_active_room_per_owner.sql` 唯一偏滤索引 + 60 个单元测试全通过
- 🟢 **房间列表接口**（T-00008）：`GET /api/v1/rooms`（公开无鉴权、分页、按 `member_count DESC, created_at DESC` 热度排序、过滤已关闭房间、含房主信息 JOIN）；78 个单元测试全通过
- 🟢 **房间详情接口**（T-00009）：`GET /api/v1/rooms/:id`（公开无鉴权、返回房主信息 + 麦位列表 MVP 为空、UUID 格式校验、404 兜底）
- 🟢 **关闭房间接口**（T-00010）：`DELETE /api/v1/rooms/:id`（JWT 鉴权、仅房主可操作、active→closed 状态变更、409 冲突检测、MVP 阶段暂不广播 WebSocket 事件）
- 🟢 **Admin 房间列表接口**（T-10004）：`GET /api/v1/admin/rooms`（Admin JWT 鉴权、`finance` 角色 403 拦截、分页 + `status` 过滤 + `keyword` 模糊搜索、可见 `closed` 房间、含房主信息 JOIN）
- 🟢 **Admin 房间详情接口**（T-10005）：`GET /api/v1/admin/rooms/:id`（Admin JWT 鉴权、`finance` 角色 403 拦截、可见 `active` 和 `closed` 状态房间、UUID 路径参数校验、软删除房间返回 404、响应含 `status` 和 `updated_at` 字段）
- 🟢 **Admin 强制关闭房间接口**（T-10006）：`DELETE /api/v1/admin/rooms/:id`（Admin JWT 鉴权、仅 `super_admin` 和 `operator` 角色有 `RoomForceClose` 权限、无 owner 检查、active→closed 状态变更、404/409 冲突检测、MVP 阶段暂不广播 WebSocket 事件）
- 🟢 **WebSocket 连接管理**（T-00011）：`GET /ws?token=<JWT>`
  - **`ws/handler.rs`**：WS 握手 + JWT 鉴权（从 query 参数 `?token=<jwt>` 提取，验证失败返回 `401 Unauthorized`）
  - **`ws/registry.rs`**：`ConnectionRegistry`（`DashMap<Uuid, ConnectionHandle>`，以 `connection_id` 为 key，解耦 `user_id`；`broadcast_to_all` 自动清除失效连接并打印 tracing 日志）
  - **`ws/heartbeat.rs`**：心跳检测 task（10s 扫描间隔，30s 超时断开；接受 `watch::Receiver<bool>` 实现优雅停机；`tokio::select!` 同时监听定时器与 shutdown 信号）
  - **`ws/connection.rs`**：单连接生命周期（`tokio::select!` 双向读写；ping→pong 回传原始 `msg_id`；`last_heartbeat` 在 ping 处理时更新）
  - **关键设计**：`connection_id` 解耦 `user_id`，同一用户第二个连接注销时仅删除自身条目，不影响已有连接；`AppState` 新增 `ws_registry: Arc<ConnectionRegistry>`
  - 13 个新增测试；全量 122 passed, 0 failed
- 🟢 **Redis 事件订阅**（T-00011B）：`admin:events` Pub/Sub 频道
  - **`events/admin_event.rs`**：`AdminEvent` enum（`BanUser` / `CloseRoom` / `BroadcastNotice`）+ serde internally-tagged（`#[serde(tag = "type", rename_all = "snake_case")]`）；S01–S04 反序列化测试含未知事件类型 Err 不 panic 验证
  - **`events/handler.rs`**：`handle_admin_event(event, registry)` — ban_user：`get_by_user_id` 取所有连接 → 发封禁通知（含 `msg_id` + `timestamp`）→ 发送 `connection_close` 指令（code=4003）→ `unregister`；close_room：两阶段（先遍历广播 RoomClosed 通知，再发送 `connection_close` 指令 code=1000 并注销）确保所有成员收到关闭消息；broadcast_notice：`registry.broadcast_to_all`；E01–E11 测试覆盖三路分支 + 离线用户不 panic + i18n_key 字段 + msg_id/timestamp 验证
  - **`events/subscriber.rs`**：`start_admin_event_subscriber(redis_url, registry, shutdown)` — 订阅 `admin:events` 频道；每条消息 `tokio::spawn` 隔离处理（单事件失败不影响主循环）；连接/订阅失败等待 2s 后重试；`tokio::select!` 监听消息流与 shutdown 信号实现优雅停机
  - **`ws/registry.rs` 扩展**：`ConnectionHandle` 新增 `room_id: Option<Uuid>`；`get_by_user_id()` 返回 `Vec<(Uuid, UnboundedSender<String>)>`（含 connection_id 用于精确注销）；新增 `get_connections_in_room(room_id)`
  - **`ws/heartbeat.rs` 扩展**（T-00042）：`close_frame_for_message()` 新增识别 `{"type":"connection_close","code":4003/1000,"reason":"..."}` JSON 指令，提取 code 和 reason 构造 WebSocket Close frame；心跳超时分支完整保留；解析失败回退为 `None`（普通文本）；4 个新增单测覆盖（code 4003/1000 + 格式错误 + 心跳回归）
  - **关键设计**：两阶段处理确保 close_room 全员收到通知；`unregister` 通过 drop sender 自然触发 WS 连接关闭；Admin 强制断连复用心跳超时 Close frame 下发机制，`connection_close` 指令在 `ws/connection.rs` 主循环出站分支识别后发送真实 Close 帧并断开；`futures-util = "0.3"` 支持 `StreamExt::next()`
  - 11 个新增测试（S01–S04 + E01–E07）+ T-00042 新增 3 个集成测试 + 4 个单元测试；全量 505 passed, 0 failed
- 🟢 **Admin 强制断连广播事件**（T-00042）：`user_banned` / `room_closed` 端到端流程
  - **Admin Server 事件发布**：`POST /admin/users/:id/ban`（T-10009）和 `DELETE /admin/rooms/:id`（T-10006）已在 DB 更新后发布 Redis `admin:events` 事件，fire-and-forget 失败不影响主流程
  - **App Server 订阅处理**：`events/handler.rs` 扩展 `connection_close_json()` 生成关闭指令（JSON 格式，含 code + reason）；`ban_user` 发送 `UserBanned` 文本帧（含 `msg_id` + `timestamp`）→ 发送 `connection_close` 指令（code=4003）→ `unregister`；`close_room` 广播 `RoomClosed`（含 `msg_id` + `timestamp`）→ 发送 `connection_close` 指令（code=1000）→ `unregister`
  - **WS 主循环集成**：`ws/connection.rs:181-193` 出站分支调用 `close_frame_for_message()`，检测到 `connection_close` 指令时构造 Close frame 并 `break` 断开连接
  - **协议 Schema**：UserBanned 通知（code=4003, reason="Account banned"）/ RoomClosed 广播（code=1000, reason="Room closed"）；通知消息均包含 `msg_id`（UUID）+ `timestamp`（epoch ms）+ `i18n_key` 字段
  - **测试覆盖**：3 个集成测试（U01-U03，验证封禁/房间关闭/多连接断开）+ 4 个单元测试（heartbeat.rs 解析逻辑）+ 11 个 handler 单测扩展（E01/E04 验证 msg_id/timestamp）；全量 505 passed（467 单元 + 3 集成 + 其他）
  - **Review Round 2 🟢 通过**：commit [1f10ec3](https://github.com/alsomail/voice-room/commit/1f10ec3)，修复 P0 BLOCKER（connection_close 指令兑现）+ P1（msg_id/timestamp 字段）+ P2（commit 原子性 + 测试覆盖）；详见 [TDS](../tds/server/T-00042.md) §五
- 🟢 **在线统计上报**（T-00011C）：`src/stats/` 模块
  - **`stats/service.rs`**：`StatsPort` trait（`user_online` / `user_offline` / `user_join_room` / `user_leave_room` / `get_online_count` / `get_active_room_count` / `take_snapshot`）；`StatsService` 真实 Redis 实现（HLL + Set + `redis::pipe().atomic()` 原子 pipeline 快照）；`FakeStatsService`（`Mutex<HashSet>` + `AtomicU32`，供单元测试注入）
  - **`stats/snapshot_task.rs`**：`snapshot_task(stats, interval_duration, shutdown)` — `tokio::time::interval` + `tokio::select!` 双路监听；快照失败仅记 `warn` 日志不退出；`shutdown.changed()` 验证返回值，`Err` 路径（sender dropped）记 `warn` 后优雅退出；`start_snapshot_task` 便捷函数供 `main.rs` 一行接入优雅停机 channel
  - **关键 Redis 数据结构**：`stats:online_users`（HyperLogLog，PFADD/PFCOUNT）、`stats:active_rooms`（Set，SADD/SREM/SCARD）、`stats:snapshot:{date}:{HH:MM}`（Hash，7 天 TTL，`redis::pipe().atomic()` 保证 HSET+EXPIRE 原子性）
  - **WS 集成**：`handle_socket` 入口调用 `stats.user_online(user_id).await.ok()`，退出调用 `stats.user_offline(user_id).await.ok()`，`.ok()` 确保失败不阻断主流程；`AppState` 新增 `stats_service: Arc<dyn StatsPort>`；测试辅助 `for_test()` 注入 `FakeStatsService`
  - 10 个新增测试（ST01–ST10，含 sender dropped 优雅退出路径 ST10）；全量 143 passed, 0 failed
- 🟢 **进入房间逻辑**（T-00012）：`src/room/` 模块
  - **`room/manager.rs`**：`RoomManager`（`DashMap<Uuid, Arc<RoomState>>`，全局房间运行时状态管理器，`get_or_create` 保证同一 room_id 只创建一个 RoomState 实例）
  - **`room/state.rs`**：`RoomState`（`members: DashMap<Uuid, MemberInfo>`、`mic_slots: RwLock<Vec<Option<Uuid>>>`）+ `MemberInfo`（user_id / nickname / avatar）
  - **`room/handler.rs`**：`handle_join_room(msg, user_id, registry, user_repo, room_repo, room_manager)` — 校验房间存在 → 用户信息查询 → 加入内存状态 → `registry.set_room_id` 关联连接 → 广播 `UserJoined` 事件 → 返回 `RoomSnapshot`
  - **`ws/registry.rs` 扩展**：新增 `set_room_id(connection_id, room_id)` 方法，将 WS 连接与房间 ID 关联
  - **`bootstrap/mod.rs` 扩展**：`AppState` 新增 `room_manager: Arc<RoomManager>`；`for_test()` 注入默认 `RoomManager::new()`
  - **关键设计**：`UserJoined` 广播给房间内所有成员（含新加入者），`RoomSnapshot` 包含完整成员列表；DB 校验 → 内存状态更新 → WS 广播三步有序执行
  - 11 个新增测试（J01–J11，含 DB 错误、房间不存在、用户不存在、成功进入广播验证）；全量 154 passed, 0 failed
- 🟢 **离开房间逻辑**（T-00013）：`src/room/` 模块扩展
  - **`room/state.rs` 扩展**：新增 `remove_from_mic_slots(user_id)` 方法 — 遍历 `mic_slots` 将该用户的麦位置为 `None`，实现自动下麦（用户离房时调用，幂等安全）
  - **`room/handler.rs` 扩展**：新增 `LeaveRoomDeps` 依赖结构体（`registry`, `room_manager`, `stats`）；新增 `do_leave_room(user_id, room_id, deps)` 核心函数（主动离房与被动断线复用同一逻辑）—— 从内存状态移除成员 → 自动下麦 → 广播 `UserLeft` 事件 → 调用 `stats.user_leave_room`；新增 `handle_leave_room(msg, user_id, deps)` 处理 WS 信令入口，提取 `room_id` 后转发给 `do_leave_room`
  - **`ws/registry.rs` 扩展**：新增 `get_room_id(connection_id)` 方法（查询当前连接绑定的房间 ID）；新增 `clear_room_id(connection_id)` 方法（断线/离房时清除绑定，防止旧关联残留）
  - **`ws/connection.rs` 扩展**：信令路由新增 `LeaveRoom` 分支调用 `handle_leave_room`；连接 cleanup 阶段（`handle_socket` 退出前）自动调用 `do_leave_room`，确保被动断线也触发退房流程
  - **关键设计**：`do_leave_room` 主被动路径复用；`clear_room_id` 防止连接复用时房间 ID 污染；`UserLeft` 广播给房间剩余成员；房间最后一个成员离开后 `RoomState` 在内存中保留（惰性清理，避免竞争）
  - 10 个新增测试（L01–L10，含自动下麦、被动断线离房、`UserLeft` 广播验证、房间不存在幂等等场景）；全量 164 passed, 0 failed
- 🟢 **上麦接口**（T-00014）：WS 信令 `TakeMic`
  - **`room/state.rs` 扩展**：新增 `TakeMicError` enum（`SlotOutOfRange` / `SlotOccupied` / `UserAlreadyOnMic` / `MicBanned`）；`RoomState` 新增 `banned_mics: DashSet<Uuid>` 字段；新增 `take_mic_slot(mic_index, user_id) -> Result<(), TakeMicError>` 同步原子方法（`mic_slots` 写锁内完成全部检查与写入，不跨 `await`，并发抢麦只有一个成功）
  - **`room/handler.rs` 扩展**：新增 `TakeMicDeps` 依赖结构体；新增 `handle_take_mic` 7 步流程：payload 解析 → 房间存在校验 → 用户在房校验 → 禁麦检查（`banned_mics`）→ `take_mic_slot` 原子占位 → 广播 `MicTaken` 事件 → 返回成功响应
  - **`ws/connection.rs` 扩展**：信令路由新增 `TakeMic` 分支，调用 `handle_take_mic`
  - **关键设计**：`take_mic_slot` 是同步函数（写锁不跨 `await`），`RwLock` 保证并发抢麦原子性；`banned_mics` 基于 `DashSet<Uuid>` 无锁并发读
  - 9 个新增测试（M01–M09，含麦位越界、麦位占用、已在麦上、禁麦、并发抢麦只有一个成功等场景）；全量 173 passed, 0 failed
- 🟢 **下麦接口**（T-00015）：WS 信令 `LeaveMic`
  - **`room/state.rs` 扩展**：新增 `leave_mic_slot(user_id) -> Option<usize>` 原子方法（`mic_slots` 写锁，遍历找到该用户的麦位置为 `None` 并返回索引；未在麦上时返回 `None`，幂等安全）
  - **`room/handler.rs` 扩展**：新增 `LeaveMicDeps` 依赖结构体（`registry`, `room_manager`）；新增 `handle_leave_mic` 5 步流程：payload 解析 → 房间存在校验 → 用户在房校验 → `leave_mic_slot` 原子释放（未在麦上直接返回成功）→ 广播 `MicLeft` 事件 → 返回成功响应；新增 `broadcast_mic_left(room_id, user_id, mic_index, registry)` 普通 fn（非 async fn，统一广播入口）；修改 `do_leave_room`：先暂存 `leave_mic_slot` 结果，`clear_room_id` 后再广播 `MicLeft`（广播顺序与 `UserLeft` 对称，避免用户还在房间时提前收到自己的 `MicLeft`）
  - **`ws/connection.rs` 扩展**：信令路由新增 `LeaveMic` 分支，调用 `handle_leave_mic`
  - **关键设计**：`leave_mic_slot` 写锁不跨 `await`，与 `take_mic_slot` 对称；`do_leave_room` 内先暂存麦位再广播，确保离房与下麦广播顺序正确；未在麦上时幂等返回成功
  - 新增测试覆盖未在麦上幂等、成功下麦广播验证、`do_leave_room` 自动下麦顺序等场景；全量 182 passed, 0 failed
- 🟢 **文本消息广播与持久化**（T-00016 + T-00043 升级版）：WS 信令 `SendMessage` + REST 历史查询
  - **T-00016 WS 信令处理**：
    - **`room/filter.rs`**（新建）：`filter_content(text) -> String` — 基于 `SENSITIVE_WORDS` 占位常量做关键词替换（`***`），为后续接入真实敏感词库预留扩展点
    - **`room/state.rs` 扩展**：新增 `muted_users: DashSet<Uuid>`（禁言用户集合）与 `processed_msg_ids: DashSet<String>`（已处理消息 ID，基于 `msg_id` 幂等去重）
    - **`room/handler.rs` 扩展**：新增 `SendMessageDeps` 依赖结构体；新增 `handle_send_message` 8 步流程：payload 解析 → 内容非空校验 → 长度限制（500 字符）→ 房间存在校验 → 禁言检查（`muted_users`）→ `msg_id` 幂等去重（`processed_msg_ids`）→ 敏感词净化（`filter_content`）→ 广播 `RoomMessage` 事件 → 返回成功响应
    - **`ws/connection.rs` 扩展**：信令路由新增 `SendMessage` 分支，调用 `handle_send_message`
    - **关键设计**：`processed_msg_ids` 基于 `DashSet<String>` 无锁并发去重，幂等插入（`insert` 返回 `false` 表示重复）；`muted_users` DashSet 无锁并发读；敏感词净化在广播前执行，广播内容为净化后文本
  - **T-00043 消息持久化 + REST 历史 API**（Round 1+2 审查通过 🟢）：
    - **数据库 Schema**（迁移 010）：`chat_messages` 表（`id UUID PK DEFAULT gen_random_uuid()`、`room_id UUID REFERENCES rooms(id) ON DELETE CASCADE`、`user_id UUID REFERENCES users(id) ON DELETE SET NULL`、`content TEXT ≤500 chars`、`created_at TIMESTAMPTZ DEFAULT NOW()`）；索引 `idx_chat_messages_room_time(room_id, created_at DESC)` 加速房间历史查询
    - **WS 持久化流程**：`handle_send_message` 步骤 7.5 敏感词净化后执行 DB 插入 → 获取 DB id (`UUID`) → 用 DB id 作 `payload.msg_id` 生成广播消息（envelope.msg_id 由 `broadcast_to_room` 独立生成，见 doc/protocol/websocket_signals.md §6.7.5）
    - **REST 历史查询接口**：`GET /api/v1/rooms/:room_id/messages?limit=&offset=`
      - **鉴权**：JWT 必需（`AuthContext` extractor）
      - **分页参数**：`limit` 默认 50/上限 100；`offset` 默认 0/软上限 100_000（超出截断，防止 PG O(N) 跳表扫描）
      - **排序与响应**：按 `created_at DESC, id DESC` 倒序；响应格式 `{ items: [{id, user_id?, nickname?, avatar_url?, content, created_at}], total, limit, offset }`
      - **数据项包含**：`user_id` + `nickname` + `avatar_url`（LEFT JOIN `users` 表）；`content` 为净化后文本（与 WS 广播一致）；`id` 与 `GET /rooms/:id/messages` 返回的 items[].id 对齐供前端去重
    - **错误码**：40003（参数非法）、40400（房间不存在）
    - **Rust 模块**：`src/modules/chat/` （controller/repository/dto/routes）；`ChatRepository` trait 含 `insert_message(room_id, user_id, content)` 与 `list_messages(room_id, limit, offset)` 两个接口
    - **测试覆盖**：R-1（迁移幂等）、U-1/U-2（单条插入+持久化）、U-3（倒序排序）、B-1（默认值与上限）、B-3（并发无丢失）；chat_persistence_test 集成覆盖；全量测试 640+ 通过
  - **T-00045 REST 发文广播修复**（BUG-CHAT-WS-BROADCAST 🟢）：
    - **背景**：客户端 REST `POST /chat-messages` 仅 INSERT 但未广播，房间内 WS 收不到消息（E2E Round 14 实证）
    - **新增端点**：`POST /api/v1/chat-messages`（`SendChatMessageRequest { room_id, content }` → `SendChatMessageResponse { msg_id }`），JWT 必需
    - **流程**：解析 room_id UUID → 校验 content（1..=500 chars，按 Unicode `chars().count()`）→ `chat_repo.insert_message` → 构造 `RoomMessage` envelope（与 WS SendMessage 路径完全对齐：顶层 `msg_id` UUID v4 + `payload.msg_id` = DB id + `timestamp` ms）→ `room_manager.get_room` 命中走 `ws::broadcaster::broadcast_to_room`（自动写 recent_broadcasts），未命中走 `broadcast_to_room_no_state`（兜底直接对该 room_id 内所有连接 fan-out）
    - **错误码**：40300（content 为空 / 超 500 chars / room_id 非合法 UUID）→ 400；缺/无效 JWT → 401
    - **测试覆盖**：`chat_rest_broadcast_test.rs` 9/9（REST-01 房间内广播、REST-02 envelope 格式、REST-03 其他房间不收、REST-04 DB 落库、REST-05 死连接容忍、REST-06 长度边界、REST-07 非法 UUID、REST-08 鉴权、REST-09 房间不在内存仍 200）
- 🟢 **钱包模块 - Schema 与迁移**（T-00017）：`src/shared/models/wallet.rs` + `app/server/migrations/004_create_wallet.sql`
  - **数据库设计**：`users` 表新增 `diamond_balance BIGINT DEFAULT 0 CHECK(>=0)` 字段；新建 `wallet_transactions` 流水表（id, user_id, type, amount, balance_after, ref_id, reason, operator_id, created_at）
  - **约束与索引**：CHECK 约束防止余额负数；复合索引 `(user_id, created_at DESC)` 支撑流水查询；`balance_after` CHECK 约束防止非法交易记录
  - **Rust 模型**：`WalletTransactionModel`（`sqlx::FromRow`）与 `WalletTxnType` enum（5 变体：gift_send/gift_receive/admin_adjust/recharge/refund）从 `shared` crate 导出；`UserModel` 新增 `diamond_balance: i64` 字段
  - **迁移幂等性**：使用 `IF NOT EXISTS` 语法三处幂等（ALTER TABLE ADD COLUMN、CREATE TABLE、CREATE INDEX），支持重复执行
  - **测试覆盖**：共 245 passed（196 server lib + 8 wallet 集成 + 41 shared 单元），覆盖 W01~W06 验收标准（幂等/默认值/CHECK 约束/索引/全类型插入）
  - **后续依赖**：T-00018（余额查询 API + WS 推送）、T-00020（SendGift 事务）等通过本数据基座实现
- 🟢 **钱包模块 - 余额查询 API + WS 推送**（T-00018）：`src/modules/wallet/` 模块完整实现
  - **HTTP 接口**：`GET /api/v1/wallet/balance`（JWT 鉴权，返回 `diamond_balance`）、`GET /api/v1/wallet/transactions?page=&size=&type=`（分页流水，按 created_at 倒序）
  - **WS 信令**：`BalanceUpdated { msg_id, diamond_balance, delta, reason, ref_id, timestamp }`，同一用户多连接全部推送
  - **WalletService 设计**：`apply_delta<'c>(&self, txn: &mut Transaction, ...)` 接受外部事务参数（行锁防并发超扣）；事务提交后调用 `notify_balance_updated` 触发推送（失败记 warn 日志）
  - **BalanceBroadcaster 设计**：`run_with_redis(rx, redis_url, registry, shutdown)` 同时监听本进程 mpsc channel 和 Redis `admin:events` PubSub，自动重连；`handle_redis_payload` 解析 Redis `balance_updated` 事件；`broadcast_event` 对所有用户连接广播，每条消息独立生成 `msg_id: Uuid::new_v4()`
  - **断线恢复**：重连时客户端主动 `GET /wallet/balance` 拉最新值（Android 在 WS `Connected` 事件触发）
  - **错误码**：401（未登录）、40003（参数非法）
  - **测试覆盖**：B01~B09 集成测试 9 个（含未登录 401、初始余额 0、分页、过滤、WS 推送延迟、多连接、Redis 事件、余额负数回滚、参数校验）；BR01~BR08 单元测试（broadcaster）；WS01~WS06 单元测试（service）；共 219 单元 + 9 集成，全部 ✅ 通过，clippy 零警告
  - **完成时间**：2025-07-15（Review Round 2 通过）
- 🟢 **礼物模块 - 配置表与列表 API**（T-00019）：`src/modules/gift/` + `app/server/migrations/005_create_gifts.sql`
  - **数据库设计**：新建 `gifts` 表（id, code, name_en, name_ar, icon_url, price, tier, effect_level, animation_url, sort_order, is_active, is_deleted, created_at, updated_at）
    - **字段约束**：`code VARCHAR(32) UNIQUE NOT NULL`（稳定标识如 'rose_01'）；`price BIGINT CHECK (>= 1)`；`tier SMALLINT CHECK (BETWEEN 1 AND 5)`；`effect_level SMALLINT DEFAULT 1`（1=none, 2=slot, 3=bottom, 4=fullscreen, 5=fullscreen+border）；`is_active BOOLEAN DEFAULT true`；`is_deleted BOOLEAN DEFAULT false`
    - **索引策略**：`idx_gifts_active_order ON gifts(tier, sort_order) WHERE is_active AND NOT is_deleted`（查询加速）
    - **种子数据**：8 款 MVP 礼物（rose_01/coffee_01/kaaba_01/camel_01/falcon_01/moon_786/castle_01/diamond_ring），price 范围 1~1314，tier 1~5 分布，幂等插入（`ON CONFLICT (code) DO NOTHING`）
  - **Rust 模型**：`GiftModel`（sqlx::FromRow，14 个字段）从 `app/shared/models/gift.rs` 导出；包含 `code` 字段支持稳定标识
  - **HTTP 接口**：`GET /api/v1/gifts/list`（无鉴权，所有用户可访问）
    - **请求**：Header `Accept-Language: ar|en`（默认 `ar`）
    - **响应**：`{code: 0, data: {items: [{id, code, name, icon_url, price, tier, effect_level, animation_url, sort_order}], version: "timestamp"}}`
    - **过滤与排序**：仅返回 `is_active=true AND is_deleted=false` 的礼物；`ORDER BY tier ASC, sort_order ASC`
  - **国际化设计**：`parse_lang_header()` 大小写不敏感，`en/en-US/en-GB` 等映射到 `"en"`，其余默认 `"ar"`；响应中 `name` 字段根据语言选择 `name_en` 或 `name_ar`
  - **缓存策略**：进程内存缓存（`tokio::sync::Mutex<HashMap<lang, (GiftListData, Instant)>>`），TTL 60s；每次 `list_active` 调用检查过期，过期则重新查库；T-10014 Admin CRUD 后调用 `invalidate_all()` 清除所有缓存
  - **架构设计**：三层依赖注入（`PgGiftRepo` 数据层 + `GiftService` 服务层 + `GiftHandler` HTTP 层）；`FakeGiftRepo` / `FakeGiftService` 用于测试替身；`call_count()` 原子计数器验证缓存命中
  - **性能目标**：响应时间 <50ms（缓存命中）
  - **代码规范**：无 unsafe/unwrap 滥用；错误处理统一走 `AppError`；`#[cfg(any(test, feature = "test-utils"))]` 严格隔离测试代码
  - **测试覆盖**：21 条集成测试（G01a~G07、G_db）验证迁移、过滤、排序、多语言、缓存命中、<50ms 响应时间；GiftModel 单元测试 10 个；repo/service/handler/shared 单元测试 33 个；共 284+ passed, 0 failed，clippy 零警告
  - **完成时间**：2025-01（TDD Agent）；审查人 claude-sonnet-4-5（AI Review Agent）
  - **已知改进方向**：
    1. **[MEDIUM] 缓存实现**：当前使用进程内存，TDS 设计为 Redis；MVP 阶段单实例可接受，多实例部署时建议切换 Redis
    2. **[MEDIUM] G04 测试覆盖**：建议在 service 单元测试补充含 `is_active=false`/`is_deleted=true` 礼物的验证场景
    3. **[LOW] version 同步**：多语言各自 miss 缓存可能生成不同时间戳，建议基于数据最新 `updated_at` 统一生成
    4. **[LOW] UUID 稳定性**：FakeGiftService 每次生成新 UUID，与缓存命中后返回相同 id 的行为不一致
- 🟢 **礼物模块 - SendGift 事务 + 广播**（T-00020）：`src/modules/gift/send_gift.rs` + `app/server/migrations/006_create_gift_records.sql`
  - **数据库设计**：新建 `gift_records` 表（id, sender_id, receiver_id, room_id, gift_id, count, total_price, msg_id, created_at）；`users` 表新增 `charm_balance BIGINT DEFAULT 0 CHECK(>=0)` 字段；幂等约束 `UNIQUE (sender_id, msg_id)`
  - **6 步强事务**：（1）幂等检查 SELECT FROM gift_records WHERE (sender_id, msg_id)；（2）数据查询（发送者房间、接收者麦位、礼物信息）；（3）BEGIN TX → 扣发送者余额（SELECT FOR UPDATE）→ 加接收者魅力值 → 写 gift_records → 更新流水 ref_id → COMMIT；（4）Redis ZINCRBY 日/周榜（非关键路径）；（5）房间广播 GiftReceived；（6）发送者推送 BalanceUpdated
  - **WS 信令**：`SendGift { gift_id, receiver_id, count, msg_id }` (C→S) → `SendGiftResult { code, gift_record_id, total_price }` (S→C)；`GiftReceived { sender, receiver, gift, count, total_price }` 房间广播；`BalanceUpdated { delta, reason: gift_send, ref_id }` 发送者推送
  - **幂等设计**：业务层先 SELECT 命中返回首次结果不重发；DB UNIQUE 约束兜底（并发同时到达）
  - **并发超扣防护**：`WalletService::apply_delta` SELECT FOR UPDATE 行锁 + UNIQUE 约束双重防线；SG10 验证 20 QPS 无超扣
  - **错误码**：40001 (INVALID_COUNT)、40002 (MISSING_PARAMS)、40290 (INSUFFICIENT_BALANCE)、40400 (SENDER_NOT_IN_ROOM)、40402 (GIFT_NOT_AVAILABLE)、40403 (RECEIVER_UNAVAILABLE)
  - **GiftReceived 完整 payload**：sender/receiver 包含 user_id/nickname/avatar；gift 包含 id/code/name/icon_url/animation_url/effect_level
  - **Rust 模型**：`GiftRecordModel` 从 `app/shared/models/gift_record.rs` 导出；`UserModel` 新增 `charm_balance: i64` 字段；`GiftSendService` 编排 6 步流程
  - **架构设计**：`send_gift.rs` 包含 `GiftSendService::send()` 核心事务逻辑、`execute_transaction()` 原子操作、`build_gift_received_msg()` 广播构造；`ranking.rs` 提供 `increment_zscore()` ZINCRBY 封装（4 个榜单）
  - **性能目标**：发送延迟 <500ms；Redis 失败不阻断主路径（仅记 warn）
  - **测试覆盖**：集成测试 SG01~SG12（12 个），单元测试 SGU01~SGU08（8 个）+ ranking RK01~RK05（5 个）+ GiftRecord 模型（5 个）；共 258 单元 + 21 gift_list + 12 send_gift 集成 = 309 total，全部通过，Clippy 零警告
  - **完成时间**：2025-06-25（TDD）→ 2025-06-26（Review Round 1 返工）→ 2025-06-27（Review Round 2 通过）
  - **Review 修复记录**（Round 2 通过）：
    1. **[C-1]** ranking.rs charm_day 双倍计数 Bug — 删除 zadd 改为纯 zincr；SG04 改精确断言
    2. **[H-1]** GiftReceived 缺失字段 — 补全 sender/receiver nickname+avatar，gift code/name/icon_url/animation_url/effect_level
    3. **[H-2]** Idempotent 死代码 — 删除 enum 变体及 handler match 臂
    4. **[H-3]** protocol.md 错误码草稿值 — 更新为实现值（40001/40002/40290/40400/40402/40403）
    5. **[M-1]** try_send 静默丢弃 — 改为 send().await，有背压，channel 关闭记 warn
    6. **[L-1]** SG08 测试污染 — 用专用测试礼物隔离数据
- 🟢 **榜单模块 - 魅力/财富榜单 API**（T-00021）：`src/modules/ranking/` 模块
  - **HTTP 接口**：`GET /api/v1/ranking?type=charm|wealth&period=day|week&limit=50`（JWT 鉴权）
    - **参数校验**：`type` 必填（charm/wealth，非法 → 40003）；`period` 可选（day/week，默认 day）；`limit` 1-100（默认 50，超范围 → 40003）
    - **响应结构**：`{type, period, period_key, items: [{rank, user_id, nickname, avatar, score, medal}], me: {rank, score}}`；Top3 medal=gold/silver/bronze；未入榜 me.rank=null, me.score=0
  - **Redis 查询**：`ZREVRANGE key 0 (limit-1) WITHSCORES` 取 Top N；`ZREVRANK` + `ZSCORE` 查 viewer 排名（1-based）
  - **批量用户信息**：`WHERE id = ANY($1)` 一次性查询所有 nickname+avatar，避免 N+1
  - **归档 Scheduler**：`do_archive_day()` / `do_archive_week()` 使用 `ZUNIONSTORE` 归档到 `ranking_archive:{type}:{period}:{date}`（7 天 TTL）；`compensate_day_archives()` 幂等补偿（读 `ranking:last_archive_{type}:day` 逐日补偿）；`start_ranking_scheduler()` tokio 每小时检查任务
  - **FakeRankingService**：返回空榜单，供 HTTP 参数校验测试（无需 Redis/PG）
  - **测试覆盖**：11 个集成测试 R01~R08（含补充 R05b/R06b/r_auth_required）+ ~20 单元测试；全量 335 tests passed, 0 failed，Clippy 零警告
  - **完成时间**：2025-06-27（TDD 实现）
- 🟢 **礼物 HTTP 端点 - POST /api/v1/gifts/send**（T-00044）：`src/modules/gift/` 模块扩展
  - **HTTP 接口**：`POST /api/v1/gifts/send`（JWT 鉴权，可选 `Idempotency-Key` header）
    - **请求体**：`{room_id, gift_id, receiver_id, count}`
    - **响应体**：`{gift_record_id, sender_balance, receiver_charm}`（成功 200）
    - **错误码**：40004 INVALID_COUNT（count ≤ 0 或 > 9999）、40290 INSUFFICIENT_BALANCE（余额不足）、40402 GIFT_NOT_AVAILABLE（礼物不存在/已下架）、40403 RECEIVER_UNAVAILABLE（接收者不在房间或不在麦上）
  - **核心设计**：
    - **复用 T-00020 核心逻辑**：WS `SendGift` 和 HTTP `POST /gifts/send` 共享同一 `GiftSendService::send()` 事务编排（扣款 → 加魅力值 → 写流水 → Redis 榜单 → 广播）
    - **幂等机制**：HTTP 支持可选 `Idempotency-Key` header（推荐客户端传 UUID），服务器以 `(sender_id, idempotency_key)` 查重；header 缺失时自动生成（无幂等保证）
    - **广播与榜单同步执行**：`broadcast_to_room()` 是 O(1) channel send 非阻塞操作（失败用 `let _ = ...` 吞掉不影响 HTTP 200）；Redis ZINCRBY 更新榜单同步执行（4 次 multiplexed RTT <5ms）；**不使用 tokio::spawn**，保持测试确定性和代码简洁性
    - **错误码与 WS 完全对齐**：40004/40290/40402/40403 与 WS `SendGiftResult` 错误码一致（doc/protocol/websocket_signals.md §6.4.2）
  - **新增文件**：`app/server/tests/send_gift_http_test.rs`（9 个集成测试 SH01~SH09）
  - **修改文件**：`gift/dto.rs`（SendGiftRequest/SendGiftResponse）、`gift/handler.rs`（send_gift_http handler）、`gift/routes.rs`（新增路由）、`gift/send_gift/types.rs`（SendGiftResult 新增 sender_new_balance/receiver_new_charm 字段）、`gift/send_gift/service.rs`（返回完整余额与魅力值）、`shared/error/code.rs`（新增 InvalidCount/GiftNotAvailable/ReceiverUnavailable）、`server/common/error.rs`（新增对应 AppError 变体）
  - **测试覆盖**：9 个 HTTP 集成测试（SH01~SH09，含成功/余额不足/参数校验/幂等性/广播失败容错）+ 12 个 WS 回归测试（SG01~SG12 保持全绿）；全量 21 passed, 0 failed
  - **完成时间**：2026-04-28（Round 1）→ 2026-04-29（Round 2 + Round 3 ✅ 通过）
  - **Review 修复记录**（Round 3 通过）：
    1. **MAJOR-2** 错误码 40001 → 40004 修正（TDS/代码/文档对齐）
    2. **MINOR-2** Idempotency-Key header 真实读取（非注释示例）
    3. **MINOR-3** 异步广播改回同步（移除不必要的 tokio::spawn，修复 sg02/sg04 回归测试时序问题）
- 🟢 **Analytics 模块 - 事件表 + HTTP 接收 API**（T-00022）：`src/core/analytics/` 模块
  - **数据库设计**：新建 `events` 分区表（id, user_id?, device_id, event_name, properties JSONB, session_id, client_ts, server_ts, app_version, os_version, locale, network_type）
    - **分区策略**：`PARTITION BY RANGE (server_ts)`，按日分区（Asia/Riyadh 时区）
    - **分区命名**：`events_YYYYMMDD`，时间范围 `[date-1 21:00 UTC, date 21:00 UTC)` = Riyadh 整天
    - **索引设计**：`idx_events_user_ts(user_id, server_ts DESC)` 用户流水查询；`idx_events_name_ts(event_name, server_ts DESC)` 事件统计
  - **EventWriter 设计**：`src/core/analytics/writer.rs`
    - **核心方法**：`persist(&self, batch: Vec<EventInput>, jwt_user_id: Option<Uuid>) -> Result<PersistResult>`
    - **校验**：device_id 必填 → 40002，batch >100 → 40204 但仍写前 100 条
    - **Properties 截断**：JSON 序列化 >8KB 截断为 `{"_truncated": true}` 并记 warn 日志
    - **JWT user_id 覆盖**：请求中 user_id 与 JWT 不一致时以 JWT 为准，记 warn 日志；无 JWT 允许 user_id=null
    - **批量写入**：`sqlx::QueryBuilder` 多行 INSERT，单请求 <200ms
  - **HTTP 接口**：`POST /api/v1/events/batch`（JWT 可选，支持未登录 Splash 阶段）
    - **请求**：`{events: [{event_name, device_id, user_id?, session_id?, client_ts?, properties, app_version?, os_version?, locale?, network_type?}]}`
    - **响应**：`{code: 0, data: {received: N, rejected_indices: [...]}}`
    - **关键特性**：兼容未登录、device_id 必填、properties 8KB 限制、批次 100 event 限制
  - **PartitionScheduler 设计**：`src/core/analytics/scheduler.rs`
    - **主动创建**：Cron `0 0 23 * * *` (Riyadh 23:00)，每日创建次日分区
    - **补偿创建**：启动时读 Redis `events:partition:last_created`，缺失 N 天分区自动补齐
    - **时间边界**：正确处理 Asia/Riyadh UTC+3 时区偏移（Bug Fix Review R2 通过）
    - **关键函数**：`create_next_partition(date)` / `compensate_missing_partitions()` / `start_partition_scheduler()`
  - **测试覆盖**：EV01~EV10 集成测试 10 个（含迁移幂等、批量写入 <200ms、properties 截断、JWT 覆盖、分区创建、补偿逻辑、并发无丢失）；W01~W13 单元测试 13 个；全量 293 passed, 0 failed
  - **完成时间**：2026-05-11（TDD 实现 + Bug 修复 Review R2 通过）
- 🟡 **Analytics 模块 - WS ReportEvent 信令**（T-00023）：WebSocket 上报通道
  - **信令设计**：`ReportEvent { events: [...] }` (C→S) → `EventReportAck { received, rejected_indices }` (S→C)
  - **复用设计**：共享 T-00022 的 EventWriter 写入服务，无重复代码
  - **user_id 处理**：来自当前 WS 连接的 JWT，覆盖客户端上报的可选 user_id，记 warn 日志
  - **server_ts**：由服务器生成，客户端 client_ts 仅作参考
  - **批量限制**：同 HTTP，100 events 上限，超过返回 BATCH_TOO_LARGE 但仍写前 100 条
  - **当前状态**：In Progress（待 TDD Agent 实现）
- 🔴 支付业务域

### 遗留技术债 (Tech Debt)
- `is_in_cooldown` / `daily_count` 两个 `SmsCodeStore` 方法当前仅供测试辅助调用，生产代码路径未使用，后续迭代可酌情清理。
- `service.rs` 中 `revoke_code` 失败时静默丢弃（`.ok()`），建议后续改为 `tracing::warn!` 记录（TDS 第五轮 Review L-01）。
- `.env.example` 中 `JWT_SECRET`、`REDIS_URL`、Twilio 相关变量需在部署文档中补充说明。
