# WebSocket 模块架构

> **关联 Task**：T-00011（WS 连接管理）、T-00011B（Redis 事件订阅）、T-00011C（在线统计上报）、T-00041（心跳超时主动断开）

## 一、模块职责

`src/ws/` 模块负责 WebSocket 长连接的全生命周期管理，包括握手鉴权、连接注册、心跳保活、信令路由与优雅断线。

## 二、核心组件

### 2.1 WS 握手（`ws/handler.rs`）

- **入口路由**：`GET /ws?token=<JWT>`
- 从 query 参数 `?token=<jwt>` 提取 JWT，调用 `shared::jwt::decode_jwt()` 验证（校验 `iss="voiceroom"`）
- 验证失败返回 `401 Unauthorized`
- 成功后升级为 WebSocket 连接，生成唯一 `connection_id: Uuid`

### 2.2 连接注册表（`ws/registry.rs`）

- **`ConnectionRegistry`**：`DashMap<Uuid, ConnectionHandle>` 以 `connection_id` 为 key
- **`ConnectionHandle`**：包含 `user_id: Uuid`、`sender: UnboundedSender<String>`、`room_id: Option<Uuid>`、`last_pong_at: Instant`
  - `last_pong_at`：记录最后收到心跳（ping）的时刻，由 connection.rs 在 ping 处理时更新，由 heartbeat.rs 定期扫描用于超时判定（T-00041）
- **关键方法**：
  - `register(connection_id, handle)` / `unregister(connection_id)` — 连接生命周期管理
  - `broadcast_to_all(message)` — 全局广播，自动清除失效连接
  - `get_by_user_id(user_id) -> Vec<(Uuid, Sender)>` — 按用户查连接（支持多连接）
  - `get_connections_in_room(room_id) -> Vec<(Uuid, Sender)>` — 按房间查连接
  - `set_room_id(connection_id, room_id)` / `get_room_id(connection_id)` / `clear_room_id(connection_id)` — 连接-房间关联管理
- **关键设计**：`connection_id` 解耦 `user_id`，同一用户第二个连接注销时仅删除自身条目，不影响已有连接

### 2.3 心跳检测与超时机制（`ws/heartbeat.rs`、`ws/connection.rs`，T-00041）

#### 设计概览
- **扫描周期**：5s（`HeartbeatConfig::scan_interval_secs`）
- **超时阈值**：30s（`HeartbeatConfig::timeout_secs`）
- **配置源**：`HeartbeatConfig { timeout_secs: 30, scan_interval_secs: 5 }`（可在运行时定制）
- **关键修复**（commit `084f91e`）：
  - heartbeat_task 此前定义但未在 main.rs spawn，导致超时检测从未执行 ➜ **现已在 `main.rs` 启动**
  - 新增 `last_pong_at` 时间戳追踪，在 ping/pong 处理时更新
  - sweeper 任务每 5s 扫描 registry，检测 `now - last_pong_at > 30s` 的连接

#### 超时触发与断开流程
1. **心跳更新**：
   - 客户端发送 `ping` 消息时，connection.rs 更新 `last_pong_at`
   - 服务端立即回复 `pong`

2. **超时检测**（heartbeat_task）：
   - 每 5s 遍历 registry 所有活跃连接
   - 检查 `Instant::now() - connection.last_pong_at > 30s`
   - 若超时，向 connection 的 sender 发送特殊信号

3. **断开动作**（connection.rs 出站分支）：
   - 收到超时信号后，向客户端发送 **Close frame**：
     ```
     Code: 1000（Normal Closure）
     Reason: "Heartbeat timeout"
     ```
   - 记录 `tracing::warn!` 日志：包含 `user_id`、`connection_id`、`elapsed_secs`、`timeout_secs`
   - 关闭 TCP socket，触发 connection cleanup（自动下麦、离房等）

#### `close_frame_for_message` 扩展：识别 `connection_close` 指令（T-00042）

**背景**：Admin 强制断连场景（用户封禁 / 房间关闭）需在 Redis 事件处理层发送关闭指令，复用心跳超时的 Close frame 下发机制。

**新增 JSON 指令识别**（`ws/heartbeat.rs:70-87`）：
```json
{
  "type": "connection_close",
  "code": 4003,  // 或 1000
  "reason": "Account banned"  // 或 "Room closed"
}
```

- **解析逻辑**：
  - 先尝试解析为 JSON，提取 `type` / `code` / `reason` 字段
  - `code` 来自 JSON 而非硬编码，支持自定义关闭码（4003 封禁 / 1000 正常关闭）
  - 解析失败时回退为 `None`（当成普通文本消息）
  - 心跳超时分支完整保留，互不干扰

- **测试覆盖**：
  - `t42_close_frame_for_connection_close_json_code_4003`：验证封禁场景
  - `t42_close_frame_for_connection_close_json_code_1000`：验证房间关闭场景
  - `t42_close_frame_for_malformed_connection_close_fallback_to_none`：格式错误回退
  - `t42_close_frame_heartbeat_timeout_still_works`：心跳分支回归验证

#### 配置化支持
- 可在启动前通过 `HeartbeatConfig` 调整参数（支持单元测试快进）
- TDD 验收覆盖：正常保活 / 边界 29s / 超时 31s / 并发场景 ✅

#### 相关源文件
| 文件 | 职责 |
|-----|------|
| `ws/heartbeat.rs` | 后台 sweeper 任务，定期扫描 registry 检测超时 |
| `ws/connection.rs` | 处理超时信号，发送 Close frame |
| `ws/registry.rs` | 存储 `last_pong_at`，供 heartbeat 任务查询 |
| `main.rs` | **[重要]** 在启动时 `tokio::spawn(heartbeat_task(...))` |

#### 参考标准
- RFC 6455 §7.4：Close Code 1000 表示"正常关闭"
- 测试用例：U-1~U-5（功能）、R-1（单元）、S-2（安全）[TDS T-00041 §三]
- Review Round 1 🟢：commit `a8c0a64` [详见 TDS 第五节]

### 2.4 单连接生命周期（`ws/connection.rs`）

- `tokio::select!` 双向读写（读 WS 消息 + 写 sender 队列）
- 信令路由：`Ping` → pong 回传原始 `msg_id`，**同时更新 `last_pong_at`**；`JoinRoom` / `LeaveRoom` / `TakeMic` / `LeaveMic` / `SendMessage` → 对应 handler
- **超时处理**（T-00041）：出站分支收到 heartbeat_task 的超时信号后，立即发送 `Message::Close(CloseFrame { code: 1000, reason: "Heartbeat timeout" })` 并退出主循环
- 退出时自动触发 `do_leave_room`（被动断线退房）

## 三、Redis 事件订阅（`src/events/`，T-00011B）

### 3.1 事件模型（`events/admin_event.rs`）

```rust
#[serde(tag = "type", rename_all = "snake_case")]
enum AdminEvent {
    BanUser { user_id: Uuid },
    CloseRoom { room_id: Uuid },
    BroadcastNotice { message: String },
}
```

### 3.2 事件处理（`events/handler.rs`）

- `ban_user`：`get_by_user_id` 取所有连接 → 发封禁通知 → `unregister` 断开
- `close_room`：两阶段处理（先遍历广播关闭消息，再遍历断开连接），确保所有成员收到通知
- `broadcast_notice`：`registry.broadcast_to_all`

#### Admin 强制断连流程（T-00042）

**端到端链路**：Admin 操作 → Redis PubSub → App Server 事件处理器 → WS 连接主循环 → 真实断开

1. **用户封禁**（`user_banned` 事件）：
   - Admin Server `POST /admin/users/:id/ban` 发布事件到 `admin:events` 频道（T-10009）
   - App Server 订阅端 `handle_admin_event` 处理：
     - `get_by_user_id(user_id)` 获取该用户所有 WS 连接（支持多连接）
     - 发送 `UserBanned` 文本帧（包含 `msg_id`、`timestamp`、`reason`、`duration_sec`）
     - 发送 `connection_close` 指令（JSON，`code=4003`）
     - `unregister(connection_id)` 注销连接
   - WS 主循环（`ws/connection.rs`）出站分支检测到 `connection_close` 指令：
     - `close_frame_for_message()` 解析 JSON 并构造 WebSocket Close frame（code=4003, reason="Account banned"）
     - 下发 Close 帧并 `break` 终止循环，真实断开连接

2. **房间关闭**（`room_closed` 事件）：
   - Admin Server `DELETE /admin/rooms/:id` 发布事件到 `admin:events` 频道（T-10006）
   - App Server 订阅端处理：
     - `get_connections_in_room(room_id)` 获取房间所有成员连接
     - 广播 `RoomClosed` 文本帧（包含 `msg_id`、`timestamp`、`reason`）
     - 给每个成员发送 `connection_close` 指令（`code=1000`）
     - `unregister` 所有连接
   - WS 主循环同样通过 `close_frame_for_message()` 识别指令，下发 Close 帧（code=1000, reason="Room closed"）

**协议 Schema**：

- **UserBanned 通知**（S→C）：
  ```json
  {
    "type": "UserBanned",
    "msg_id": "uuid",
    "timestamp": 1720000000000,
    "i18n_key": "notification.user_banned",
    "message": "{reason} 封禁 {duration_sec}s"
  }
  ```

- **RoomClosed 广播**（S→房间）：
  ```json
  {
    "type": "RoomClosed",
    "msg_id": "uuid",
    "timestamp": 1720000000000,
    "i18n_key": "notification.room_closed",
    "message": "房间已关闭：{reason}"
  }
  ```

- **connection_close 指令**（内部消息，非公开协议）：
  ```json
  {
    "type": "connection_close",
    "code": 4003,  // 封禁: 4003, 房间关闭: 1000
    "reason": "Account banned"
  }
  ```

**测试覆盖**：
- **单元测试**（`events/handler.rs`）：E01-E11，验证通知格式、msg_id/timestamp 字段、幂等性
- **集成测试**（`admin_force_disconnect_test.rs`）：U01-U03，验证多连接场景全部断开
- **回归保证**：voice-room-server 467 单测 + 3 集成测试全通过

### 3.3 订阅者（`events/subscriber.rs`）

- 订阅 Redis `admin:events` 频道
- 每条消息 `tokio::spawn` 隔离处理（单事件失败不影响主循环）
- 连接/订阅失败等待 2s 后重试
- `tokio::select!` 监听消息流与 shutdown 信号实现优雅停机

## 四、在线统计上报（`src/stats/`，T-00011C）

### 4.1 核心接口（`stats/service.rs`）

- **`StatsPort` trait**：`user_online` / `user_offline` / `user_join_room` / `user_leave_room` / `get_online_count` / `get_active_room_count` / `take_snapshot`
- **`StatsService`**：真实 Redis 实现（HLL + Set + `redis::pipe().atomic()` 原子 pipeline 快照）
- **`FakeStatsService`**：`Mutex<HashSet>` + `AtomicU32`，供单元测试注入

### 4.2 Redis 数据结构

| Key | 类型 | 用途 |
|-----|------|------|
| `stats:online_users` | HyperLogLog | PFADD/PFCOUNT 在线用户近似计数 |
| `stats:active_rooms` | Set | SADD/SREM/SCARD 活跃房间精确计数 |
| `stats:snapshot:{date}:{HH:MM}` | Hash | 定时快照，7 天 TTL |

### 4.3 快照定时任务（`stats/snapshot_task.rs`）

- `tokio::time::interval` + `tokio::select!` 双路监听（定时器 + shutdown）
- 快照失败仅记 `warn` 日志不退出
- `shutdown.changed()` 验证返回值，sender dropped 时优雅退出

### 4.4 WS 集成

- `handle_socket` 入口调用 `stats.user_online(user_id).await.ok()`
- 退出调用 `stats.user_offline(user_id).await.ok()`
- `.ok()` 确保统计失败不阻断主流程

## 五、测试覆盖

| 模块 | 测试数 | 说明 |
|------|--------|------|
| WS 连接管理（T-00011） | 13 | 握手、注册、心跳、信令路由 |
| WS 心跳超时（T-00041） | 6 | 正常保活、30s 超时、边界值 29/31s、并发场景、close frame 映射 |
| Redis 事件订阅（T-00011B） | 11 | S01-S04 反序列化 + E01-E07 三路事件处理 |
| 在线统计（T-00011C） | 10 | ST01-ST10 含 sender dropped 优雅退出 |
| **合计** | **40** | — |
