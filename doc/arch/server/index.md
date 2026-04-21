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
Server 端基于 Rust + Axum 构建。启动骨架（配置、日志、健康检查）已完成；Auth 业务域（短信验证码、手机号登录、JWT 鉴权、用户信息）已全部落地并通过 Review；数据库（SQLx 0.8 + PostgreSQL）与 Redis 已接入运行链路；Room 业务域数据层（`rooms` 表 DDL + `RoomModel` struct，T-00006）已完成；**创建房间接口**（`POST /api/v1/rooms`，T-00007）已落地（含 JWT 鉴权、参数校验、bcrypt 密码哈希、唯一 active 房间约束，60 个单元测试全通过）；**房间列表接口**（`GET /api/v1/rooms`，T-00008）已落地（分页热度排序、无鉴权，78 个测试全通过）；**房间详情接口**（`GET /api/v1/rooms/:id`，T-00009）已落地（公开无鉴权、UUID 路径参数校验、返回房主信息与麦位列表 MVP 为空）；**关闭房间接口**（`DELETE /api/v1/rooms/:id`，T-00010）已落地（JWT 鉴权、仅房主可操作、active→closed 状态变更、409 冲突检测，MVP 阶段暂不广播 WebSocket 事件）；**Admin 房间列表接口**（`GET /api/v1/admin/rooms`，T-10004）已落地（Admin JWT 鉴权、finance 角色 403 拦截、分页 + 状态过滤 + 关键词搜索、可见 closed 房间）；**Admin 房间详情接口**（`GET /api/v1/admin/rooms/:id`，T-10005）已落地（Admin JWT 鉴权、finance 角色 403 拦截、可见 active/closed 状态房间、UUID 路径参数校验、响应含 status 与 updated_at 字段）；**Admin 强制关闭房间接口**（`DELETE /api/v1/admin/rooms/:id`，T-10006）已落地（Admin JWT 鉴权、仅 super_admin/operator 角色有 RoomForceClose 权限、无 owner 检查、active→closed 状态变更、404/409 冲突检测，MVP 阶段暂不广播 WebSocket 事件）；**WebSocket 连接管理**（`GET /ws?token=<JWT>`，T-00011）已落地（`src/ws/` 模块，JWT 握手鉴权、`ConnectionRegistry` DashMap 无锁并发注册、心跳检测 task 支持优雅停机、tokio::select! 双向读写、`connection_id` 解耦 `user_id` 防多连接注销竞争，13 个测试全通过，全量 122 passed）；**Redis 事件订阅**（`admin:events` 频道，T-00011B）已落地（`src/events/` 模块，`AdminEvent` serde internally-tagged 反序列化、`handle_admin_event` 三路事件处理（ban_user/close_room/broadcast_notice）、`tokio::spawn` 隔离单事件失败、Redis Pub/Sub 自动重连 task + shutdown 优雅停机、`ConnectionRegistry` 扩展 `room_id: Option<Uuid>` + `get_by_user_id()` + `get_connections_in_room()`，11 个新增测试，全量 133 passed, 0 failed）；**在线统计上报**（T-00011C）已落地（`src/stats/` 模块，`StatsPort` trait + `StatsService` Redis 实现（HLL/Set/原子 pipeline）+ `FakeStatsService` 测试替身、`snapshot_task` 每 60s 定时快照支持 shutdown watch channel 优雅停机、WS handle_socket 入口/退出调用 user_online/user_offline 失败 .ok() 不阻断主流程，10 个新增测试，全量 143 passed, 0 failed）；**进入房间逻辑**（T-00012）已落地（`src/room/` 模块，`RoomManager` DashMap 全局内存状态、`RoomState` 成员表与麦位、`handle_join_room` WS 信令处理、`registry.set_room_id` 关联连接与房间、DB 校验→内存更新→广播 `UserJoined` 有序三步，11 个新增测试，全量 154 passed, 0 failed）；**离开房间逻辑**（T-00013）已落地（`src/room/` 模块扩展，`room/state.rs` 新增 `remove_from_mic_slots(user_id)` 自动下麦、`room/handler.rs` 新增 `do_leave_room` 主被动路径复用逻辑与 `handle_leave_room` WS 信令入口、`ws/registry.rs` 新增 `get_room_id` / `clear_room_id` 方法、`ws/connection.rs` LeaveRoom 信令路由与断线自动触发退房，10 个新增测试，全量 164 passed, 0 failed）；**上麦接口**（T-00014）已落地（`room/state.rs` 新增 `TakeMicError` enum、`banned_mics: DashSet<Uuid>`、`take_mic_slot` 同步原子方法（写锁不跨 await，并发抢麦只有一个成功）、`room/handler.rs` 新增 `TakeMicDeps` 与 `handle_take_mic` 7 步流程（payload 解析→房间校验→禁麦检查→原子占位→广播 `MicTaken`→响应）、`ws/connection.rs` TakeMic 信令路由分支，9 个新增测试，全量 173 passed, 0 failed）；**下麦接口**（T-00015）已落地（`room/state.rs` 新增 `leave_mic_slot(user_id) -> Option<usize>` 原子方法（写锁不跨 await）、`room/handler.rs` 新增 `LeaveMicDeps`、`handle_leave_mic` 5 步流程、`broadcast_mic_left` 普通 fn、`do_leave_room` 先暂存麦位再广播保证顺序、`ws/connection.rs` LeaveMic 信令路由分支，全量 182 passed, 0 failed）；**文本消息广播**（T-00016）已落地（`room/filter.rs` 新建 `filter_content` 敏感词净化、`room/state.rs` 新增 `muted_users: DashSet<Uuid>` 与 `processed_msg_ids: DashSet<String>`、`room/handler.rs` 新增 `SendMessageDeps` 与 `handle_send_message` 8 步流程（内容校验→长度限制→房间校验→禁言检查→幂等去重→净化→广播→响应）、`ws/connection.rs` SendMessage 信令路由分支，全量 196 passed, 0 failed）；支付业务域仍未展开。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：请先通过以下子文档确认“当前已实现的骨架”和“尚未落地的业务边界”，再决定是否继续扩展。

### 实际目录：
- 🧱 [启动、配置与目录结构](./structure.md) - `main.rs`、`bootstrap`、`config`、`logging`、数据库 / Redis 初始化与测试入口现状。
- 📊 [能力状态与缺口盘点](./status.md) - 现有可用能力、未落地模块与下一步约束。
- 🔐 [Auth 模块架构](./auth.md) - 短信验证码（T-00002）、手机号登录（T-00003）、JWT 中间件（T-00004）、获取用户信息（T-00005）的路由、服务、Redis Key 设计与错误码映射。
- 🗄️ [数据库 Schema 设计](./database.md) - 各业务表 DDL 说明、字段约束、索引策略与 Rust 模型映射（含 `rooms` 表，T-00006）。
- 🔌 [WebSocket 模块架构](./websocket.md) - WS 握手鉴权（T-00011）、`ConnectionRegistry`、心跳检测、单连接生命周期与 `connection_id` 解耦设计。
- 🏠 [房间运行时模块](./room_runtime.md) - `src/room/` 模块说明：`RoomManager`（DashMap 全局状态）、`RoomState`（成员表 + 麦位 + `banned_mics` + `muted_users` + `processed_msg_ids`）、`handle_join_room` WS 信令处理（T-00012）、`do_leave_room` / `handle_leave_room` 离开房间逻辑（T-00013）、`take_mic_slot` / `handle_take_mic` 上麦逻辑（T-00014）、`leave_mic_slot` / `handle_leave_mic` 下麦逻辑（T-00015）、`filter_content` 敏感词净化 / `handle_send_message` 文本消息广播（T-00016）。

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🟢 Server 启动装配、优雅停机与 Axum 路由注册
- 🟢 `GET /ping` 健康检查、JSON 响应与 `x-request-id`
- 🟢 tracing 初始化、请求级 span 与访问日志字段注入
- 🟢 `app/shared` crate 集成（JWT encode/decode + iss 校验、bcrypt 密码工具、公共错误码）
- 🟢 配置分层读取（`.env` + `config/*.toml` + 环境变量覆盖）
- 🟢 数据库连接池（SQLx 0.8 + PostgreSQL）与自动 migration（`sqlx::migrate!`）
- 🟢 Redis 连接（`MultiplexedConnection` 缓存复用）
- 🟢 **Auth 模块**：`POST /api/v1/auth/verification-codes`（T-00002）、`POST /api/v1/auth/login`（T-00003）、JWT 鉴权中间件（T-00004）、`GET /api/v1/users/me`（T-00005）
- 🟢 SMS 防腐层（`SmsProvider` trait）：生产用 Twilio，开发/CI 用 Mock
- 🟢 统一错误响应结构（含 `request_id`、`safe_message` 防信息泄露）
- 🟢 **数据层 — rooms 表**（T-00006）：`002_create_rooms.sql` DDL（6 个 CHECK 约束、3 个索引含软删除偏滤）+ `RoomModel` struct（29 个单元测试全通过）
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
  - **`events/handler.rs`**：`handle_admin_event(event, registry)` — ban_user：`get_by_user_id` 取所有连接 → 发封禁通知 → `unregister`；close_room：两阶段（先遍历广播再遍历断开）确保所有成员收到关闭消息；broadcast_notice：`registry.broadcast_to_all`；E01–E07 测试覆盖三路分支 + 离线用户不 panic + 容错
  - **`events/subscriber.rs`**：`start_admin_event_subscriber(redis_url, registry, shutdown)` — 订阅 `admin:events` 频道；每条消息 `tokio::spawn` 隔离处理（单事件失败不影响主循环）；连接/订阅失败等待 2s 后重试；`tokio::select!` 监听消息流与 shutdown 信号实现优雅停机
  - **`ws/registry.rs` 扩展**：`ConnectionHandle` 新增 `room_id: Option<Uuid>`；`get_by_user_id()` 返回 `Vec<(Uuid, UnboundedSender<String>)>`（含 connection_id 用于精确注销）；新增 `get_connections_in_room(room_id)`
  - **关键设计**：两阶段处理确保 close_room 全员收到通知；`unregister` 通过 drop sender 自然触发 WS 连接关闭；`futures-util = "0.3"` 支持 `StreamExt::next()`
  - 11 个新增测试（S01–S04 + E01–E07）；全量 133 passed, 0 failed
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
- 🟢 **文本消息广播**（T-00016）：WS 信令 `SendMessage`
  - **`room/filter.rs`**（新建）：`filter_content(text) -> String` — 基于 `SENSITIVE_WORDS` 占位常量做关键词替换（`***`），为后续接入真实敏感词库预留扩展点
  - **`room/state.rs` 扩展**：新增 `muted_users: DashSet<Uuid>`（禁言用户集合）与 `processed_msg_ids: DashSet<String>`（已处理消息 ID，基于 `msg_id` 幂等去重）
  - **`room/handler.rs` 扩展**：新增 `SendMessageDeps` 依赖结构体；新增 `handle_send_message` 8 步流程：payload 解析 → 内容非空校验 → 长度限制（500 字符）→ 房间存在校验 → 禁言检查（`muted_users`）→ `msg_id` 幂等去重（`processed_msg_ids`）→ 敏感词净化（`filter_content`）→ 广播 `MessageReceived` 事件 → 返回成功响应
  - **`ws/connection.rs` 扩展**：信令路由新增 `SendMessage` 分支，调用 `handle_send_message`
  - **关键设计**：`processed_msg_ids` 基于 `DashSet<String>` 无锁并发去重，幂等插入（`insert` 返回 `false` 表示重复）；`muted_users` DashSet 无锁并发读；敏感词净化在广播前执行，广播内容为净化后文本；全量 196 passed, 0 failed
- 🟢 **钱包模块 - Schema 与迁移**（T-00017）：`src/shared/models/wallet.rs` + `app/server/migrations/004_create_wallet.sql`
  - **数据库设计**：`users` 表新增 `diamond_balance BIGINT DEFAULT 0 CHECK(>=0)` 字段；新建 `wallet_transactions` 流水表（id, user_id, type, amount, balance_after, ref_id, reason, operator_id, created_at）
  - **约束与索引**：CHECK 约束防止余额负数；复合索引 `(user_id, created_at DESC)` 支撑流水查询；`balance_after` CHECK 约束防止非法交易记录
  - **Rust 模型**：`WalletTransactionModel`（`sqlx::FromRow`）与 `WalletTxnType` enum（5 变体：gift_send/gift_receive/admin_adjust/recharge/refund）从 `shared` crate 导出；`UserModel` 新增 `diamond_balance: i64` 字段
  - **迁移幂等性**：使用 `IF NOT EXISTS` 语法三处幂等（ALTER TABLE ADD COLUMN、CREATE TABLE、CREATE INDEX），支持重复执行
  - **测试覆盖**：共 245 passed（196 server lib + 8 wallet 集成 + 41 shared 单元），覆盖 W01~W06 验收标准（幂等/默认值/CHECK 约束/索引/全类型插入）
  - **后续依赖**：T-00018（余额查询 API + WS 推送）、T-00020（SendGift 事务）等通过本数据基座实现
- 🔴 支付业务域

### 遗留技术债 (Tech Debt)
- `is_in_cooldown` / `daily_count` 两个 `SmsCodeStore` 方法当前仅供测试辅助调用，生产代码路径未使用，后续迭代可酌情清理。
- `service.rs` 中 `revoke_code` 失败时静默丢弃（`.ok()`），建议后续改为 `tracing::warn!` 记录（TDS 第五轮 Review L-01）。
- `.env.example` 中 `JWT_SECRET`、`REDIS_URL`、Twilio 相关变量需在部署文档中补充说明。
