# 六、WebSocket 信令（预留）

> 将在模块3 WebSocket 连接管理（T-00012）实现时正式定义。以下为设计预留。

## 6.1 连接建立

```
ws://host/ws?token=<JWT>
```

## 6.2 心跳

- 客户端每 15 秒发送 `{"type":"ping"}`
- 服务端回复 `{"type":"pong"}`
- 30 秒无心跳自动断开

## 6.3 消息通用格式

```json
{
  "type": "EventType",
  "msg_id": "uuid",
  "payload": {},
  "timestamp": 1713312000
}
```

响应/ACK 通用格式：
```json
{ "type": "XxxResult", "msg_id": "uuid", "code": 0, "payload": { } }
```

---

## 6.4 Phase 1 (E-07) 礼物经济信令

| 信令 | 方向 | 关联 Task | 详细定义 |
|------|------|-----------|----------|
| `BalanceUpdated` | S→C | T-00018 | [tds/server/T-00018.md](../tds/server/T-00018.md) |
| `SendGift` / `SendGiftResult` | C↔S | T-00020 | [tds/server/T-00020.md](../tds/server/T-00020.md) |
| `GiftReceived` | S→房间广播 | T-00020 | 同上 |

### 6.4.1 BalanceUpdated（S→C）

**更新日期**：T-00018 Review Round 2（对齐实现 + WS 通用格式 §6.3）

```json
{
  "type": "BalanceUpdated",
  "msg_id": "uuid",
  "payload": {
    "diamond_balance": 4800,
    "delta": -520,
    "reason": "gift_send",
    "ref_id": "uuid|null"
  },
  "timestamp": 1720000000000
}
```

**字段说明**：
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定值 `"BalanceUpdated"` |
| `msg_id` | string (UUID) | ✅ | 每条推送独立生成，符合 §6.3 通用格式 |
| `payload.diamond_balance` | int64 | ✅ | 变更后的钻石余额 |
| `payload.delta` | int64 | ✅ | 本次变化量（正数=充值/收礼，负数=扣款/送礼） |
| `payload.reason` | string | ✅ | 变化原因：`"gift_send"` / `"gift_receive"` / `"admin_adjust"` / `"recharge"` / `"refund"` |
| `payload.ref_id` | string (UUID) \| null | | 关联业务 ID（礼物记录 ID 或 admin_log_id），可选 |
| `timestamp` | int64 (ms) | ✅ | 服务端推送时间戳（毫秒） |

**推送时机**：`WalletService.apply_delta()` 事务提交成功后，通过 `notify_balance_updated` 触发本进程推送；Admin 服务通过 Redis PUBLISH `admin:events` 触发跨进程推送。

**多端在线**：同一用户多个 WS 会话均会收到推送，每条消息有独立 `msg_id`。

### 6.4.2 SendGift（C→S）
```json
{ "type":"SendGift", "msg_id":"uuid",
  "payload":{ "room_id":"uuid","gift_id":"uuid","receiver_id":"uuid","count":1 } }
```
**错误码**（更新日期：T-00020 Review Round 1，与 TDS/实现对齐）：

| code | 常量名 | 含义 |
|------|--------|------|
| `40001` | INVALID_COUNT | count 为 0 或超过 9999 |
| `40002` | MISSING_PARAMS | 参数缺失或格式非法 |
| `40290` | INSUFFICIENT_BALANCE | 发送者钻石余额不足 |
| `40400` | SENDER_NOT_IN_ROOM | 发送者不在指定房间 |
| `40402` | GIFT_NOT_AVAILABLE | 礼物不存在或已下架 |
| `40403` | RECEIVER_UNAVAILABLE | 接收者不在房间或不在麦上 |

### 6.4.3 GiftReceived（S→房间）
```json
{
  "type": "GiftReceived",
  "msg_id": "uuid",
  "payload": {
    "gift_record_id": "uuid",
    "sender": { "user_id":"uuid", "nickname":"Alice", "avatar":"https://..." },
    "receiver": { "user_id":"uuid", "nickname":"Bob", "avatar":null },
    "gift": {
      "id":"uuid", "code":"castle_01", "name":"قصر",
      "icon_url":"https://...", "animation_url":"https://...", "effect_level":4
    },
    "count": 1,
    "total_price": 520
  },
  "timestamp": 1720000000000
}
```

---

## 6.5 Phase 1 (E-07.5) 埋点上报信令

| 信令 | 方向 | 关联 Task | 详细定义 |
|------|------|-----------|----------|
| `ReportEvent` / `EventReportAck` | C↔S | T-00023 | [tds/server/T-00023.md](../tds/server/T-00023.md) |

```json
{ "type":"ReportEvent", "msg_id":"uuid",
  "payload":{ "events":[{ /* 见 T-00022 事件结构 */ }] } }
```
ACK：`{ type:"EventReportAck", msg_id, code:0, payload:{ received:98, rejected_indices:[12,45] } }`
错误码：`40204` BATCH_TOO_LARGE（仍写前 100 条）。

---

## 6.6 Phase 1.5 (E-10) 房间治理信令

| 信令 | 方向 | 关联 Task | 详细定义 |
|------|------|-----------|----------|
| `RoomInfoUpdated` | S→房间 | T-00025 | [tds/server/T-00025.md](../tds/server/T-00025.md) |
| `KickUser` / `KickUserResult` | C↔S | T-00028 | [tds/server/T-00028.md](../tds/server/T-00028.md) |
| `UserKicked` | S→被踢者 | T-00028 | 同上 |
| `MuteUser` / `UnmuteUser` | C→S | T-00029 | [tds/server/T-00029.md](../tds/server/T-00029.md) |
| `UserMuted` | S→房间 | T-00029 | 同上（duration_sec=0 表示解除）|
| `TransferAdmin` | C→S | T-00030 | [tds/server/T-00030.md](../tds/server/T-00030.md) |
| `AdminChanged` | S→房间 | T-00030 | 同上 |
| `ForceTakeMic` / `ForceLeaveMic` | C→S | T-00030 | 同上（复用 `MicTaken/MicLeft` 广播 + `forced_by` 字段） |

### 6.6.1 统一错误码（本 Epic 新增）
| code | 含义 |
|------|------|
| 40104 | PASSWORD_REQUIRED（密码房无 token） |
| 40105 | TOKEN_EXPIRED（room_access token 过期） |
| 40301 | PERMISSION_DENIED |
| 40302 | CANNOT_KICK_OWNER / CANNOT_TARGET_OWNER |
| 40305 | CHAT_MUTED（SendMessage 被拒） |
| 40306 | MIC_MUTED（TakeMic 被拒） |
| 40404 | MIC_NOT_FOUND（ForceLeaveMic 目标不在麦） |
| 42910 | PASSWORD_LOCKED（5 次错误锁定 30min） |
| 42911 | KICKED_COOLDOWN（10min 冷却中） |

### 6.6.2 JoinRoom 扩展（密码房）
JoinRoom payload 增 `access_token?: string`（密码房必填，从 `POST /rooms/:id/verify-password` 获取，TTL 60s）。

### 6.7 重连续传（last_msg_id 回放，T-审计 P1-6）

#### 6.7.1 客户端契约
客户端在 `JoinRoom.payload` 中可选携带 `last_msg_id?: string`：
- **取值**：客户端最近一次成功收到的房间广播 envelope 上的 `msg_id`（服务端 UUID v4）。
- **场景**：网络抖动 / 后台切回 / 主动重连等导致 WebSocket 短暂断开，重连后通过此字段请求服务端补发断连期间错过的广播。
- **首次进房 / 不需要续传**：省略该字段或传空字符串，行为与传统 `JoinRoom` 一致。

#### 6.7.2 服务端行为
1. 服务端为每条房间广播 envelope（`UserJoined / UserLeft / MicTaken / MicLeft / RoomMessage / GiftReceived / UserMuted / AdminChanged / UserKicked` 等）注入唯一 `msg_id` 字段（UUID v4），并写入 `RoomState.recent_broadcasts` 环缓冲。
2. 当 `JoinRoom` 携带 `last_msg_id`：
   - **命中**（`last_msg_id` 仍在缓冲窗口内）：把 `(last_msg_id, now]` 区间的所有广播原样**点对点**推送给该 connection（仅该连接，不重新广播）。回放消息**不**再次写入 `recent_broadcasts`。
   - **出窗**（`last_msg_id` 已被驱逐 / 服务端重启 / 客户端伪造）：不回放，仅记录 `tracing::info`。客户端应主动调用 `GET /rooms/:id` / `GET /rooms/:id/messages` 等 REST 接口拉取兜底数据。
   - **缓冲为空**（房间刚启动）：等同于"出窗"。
3. 回放时序在 `JoinRoom` 流程中位于"获取/创建 room_state 之后、加入成员表 / 广播 UserJoined 之前"——确保自己加入产生的 `UserJoined` 不会出现在自己的回放结果中。

#### 6.7.3 容量与 SLO
- **缓冲容量**：每个房间 200 条（FIFO，最旧驱逐）。按平均 ~1KB/条估算，单房间内存上限 ~200KB。
- **覆盖断连窗口**：在峰值 6 QPS 房间广播下覆盖 ≥30 s 断网；典型房间（<1 QPS）覆盖 ≥3 min。
- **out-of-order**：FIFO push 顺序即客户端可重放顺序。

#### 6.7.4 不参与回放的信令
以下消息**不**走 `recent_broadcasts`，因此不可被 `last_msg_id` 续传：
- `RoomClosed` / 治理通告类：管理员触发后立即断开连接，本身不在 in-room 流程内。
- 点对点信令（`UserKicked` 直接发给被踢者、JoinRoom 本身的 ack 等）。
- `BalanceUpdated` 等用户级（非房间级）推送。

客户端如需保证看到这些事件，应通过 REST 接口（房间状态、用户余额）轮询兜底。

#### 6.7.5 envelope.msg_id vs payload.msg_id（双 ID 职责分裂，T-00043 引入）

在 `RoomMessage` 等聊天广播中，**两个 `msg_id` 字段并存**且语义不同，客户端需按用途区分：

| 字段 | 来源 | 用途 | 稳定性 |
|------|------|------|------|
| `msg_id`（envelope 顶层） | 服务端 `broadcast_to_room` 注入的 **UUID v4**（每次推送独立生成） | §6.7 `last_msg_id` 重连续传游标，配合 `recent_broadcasts` 环缓冲 | 仅在缓冲窗口内有效；超出窗口 / 服务端重启后失效 |
| `payload.msg_id` | `chat_messages.id`（**DB 行主键**，T-00043 落库返回） | 业务级稳定标识：用于 REST `GET /api/v1/rooms/:id/messages` 中的去重 / 锚定 / 引用回复等 | 永久（除非该消息被删除） |

**客户端约定**：
- **重连续传**：`JoinRoom.last_msg_id` 必须传 envelope 顶层 `msg_id`；**不要**传 `payload.msg_id`（DB id 不在缓冲索引里，永远视作"出窗"）。
- **历史 REST 比对 / 去重**：以 `payload.msg_id` 为准（与 `GET /rooms/:id/messages` 返回的 `items[].id` 直接对齐）。
- **其他广播**（如 `UserJoined / GiftReceived`）目前仅有 envelope `msg_id`；`payload.msg_id` 仅在 chat 类业务消息中存在。

---

## 6.8 Chat 文本消息信令（T-00016 / T-00043 / T-00045 / T-00047）

> **关联 Task**：T-00016（WS 广播 MVP）· T-00043（持久化 + REST 历史）· T-00045（REST 写入广播闭环）· T-00047（协议路径绑定首发，本节落锚）
>
> **协议路径**（**唯一事实源**）：客户端**主路径** ⭐ 走 WS `SendMessage`；REST `POST /api/v1/chat-messages`（[room_api.md §3.6](./room_api.md)）为运营 / 兜底备路径，二者产生的 `RoomMessage` envelope **形态等价**（详见 §6.8.3）。
>
> **客户端实现绑定（grep-able）**：Android 端**唯一**调用入口 `RoomViewModel.sendMessage` → `wsClient.sendEnvelope(type = "SendMessage", ...)`；**严禁**走 Retrofit `POST /chat-messages`。

### 6.8.1 SendMessage（C→S）

**方向**：客户端 → 服务端
**服务端处理**：`app/server/src/room/handler/chat.rs::handle_send_message`
**前置条件**：客户端必须已通过 `JoinRoom` 加入目标房间（连接 ↔ room_id 绑定）。

```json
{
  "type": "SendMessage",
  "msg_id": "<client-uuid-v4>",
  "payload": {
    "content": "<1..=500 Unicode chars，去前后空白后非空>"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"SendMessage"` |
| `msg_id` | string (UUID v4) | ✅ | **客户端生成**，用于 ACK 关联 + 服务端 `processed_msg_ids` 幂等去重；同 `msg_id` 重发不会二次广播 |
| `payload.content` | string | ✅ | 文本内容；按 Unicode `chars().count()` 计长，1–500；空串 / 全空白拒绝 |
| `timestamp` | int64 (ms) | | 客户端时间戳，仅日志用，不参与业务 |

**ACK：`SendMessageResult`（S→C，单播）**

```json
{
  "type": "SendMessageResult",
  "msg_id": "<原始 client-uuid-v4，回显>",
  "code": 0,
  "message": "ok",
  "timestamp": 1720000000000
}
```

**错误码**：

| code | 常量 | 含义 |
|------|------|------|
| `0` | OK | 成功（含幂等命中） |
| `40001` | CONTENT_TOO_LONG | content 超过 500 字符 |
| `40002` | MISSING_PARAMS | content 缺失或空字符串 |
| `40303` | USER_BANNED | 用户被全局封禁 |
| `40305` | CHAT_MUTED | 用户在房间内被禁言（见 §6.6.1） |
| `40400` | NOT_IN_ROOM / ROOM_NOT_FOUND | 当前连接未加入房间 / 房间状态不存在 |
| `50000` | DB_PERSIST_FAILED | DB 写入失败（不广播） |

### 6.8.2 RoomMessage（S→Room 广播）

**方向**：服务端 → 房间内所有 WS 连接（含发送者）
**触发源**：WS `SendMessage` 成功（§6.8.1）**或** REST `POST /api/v1/chat-messages` 成功（[room_api.md §3.6](./room_api.md)）。两条路径产生的 envelope **逐字段等价**（除 `envelope.msg_id` 各自独立 UUID v4，详见 §6.8.3 PROTO-2）。
**广播实现**：`app/server/src/ws/broadcaster.rs::broadcast_to_room`

```json
{
  "type": "RoomMessage",
  "msg_id": "<server-uuid-v4，envelope 续传游标，§6.7.5>",
  "payload": {
    "msg_id": "<chat_messages.id，DB UUID v4，T-00043>",
    "user_id": "<sender uuid>",
    "content": "<filter_content() 敏感词净化后的文本>"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"RoomMessage"` |
| `msg_id`（envelope） | string (UUID v4) | ✅ | 服务端 `broadcast_to_room` 注入；写入 `RoomState.recent_broadcasts` 用于 §6.7 重连续传 |
| `payload.msg_id` | string (UUID v4) | ✅ | DB 行主键（`chat_messages.id`），永久稳定标识，与 REST 历史 `GET /rooms/:id/messages` 的 `items[].id` 对齐 |
| `payload.user_id` | string (UUID v4) | ✅ | 发送者用户 ID |
| `payload.content` | string | ✅ | **敏感词过滤后**的内容（与 DB 落库内容一致） |
| `timestamp` | int64 (ms) | ✅ | 服务端广播时刻 |

### 6.8.3 双路径等价契约（PROTO-2，T-00047）

> **铁律**：WS `SendMessage` 与 REST `POST /api/v1/chat-messages` 写入的同一 `(room_id, user_id, content)` 必须在房间内产生**逐字段相等**的 `RoomMessage` envelope，**仅** `envelope.msg_id` 例外（两条路径各自独立 UUID v4）。

| 字段 | WS 路径 | REST 路径 | 等价 |
|------|---------|-----------|------|
| `type` | `"RoomMessage"` | `"RoomMessage"` | ✅ |
| `msg_id`（envelope） | server UUID v4 | server UUID v4（不同实例） | ❌（按设计不等） |
| `payload.msg_id` | DB row id（优先）/ client msg_id（旧路径兜底） | DB row id | ✅（要求 REST 必须等待 DB 提交后才广播） |
| `payload.user_id` | sender uuid | sender uuid（来自 JWT） | ✅ |
| `payload.content` | `filter_content(原文)` | `filter_content(原文)` ⚠️ | ✅（**REST 路径需补齐敏感词过滤**，详见 T-00047 实现差异修复） |
| `timestamp` | server `Utc::now().timestamp_millis()` | server `Utc::now().timestamp_millis()` | ✅（值不同，但语义/类型/单位等价） |

**实施差异（T-00047 修复点）**：当前 REST 路径（T-00045 落地）**未**调用 `filter_content`、**未**走 `processed_msg_ids` 幂等、**未**做 `muted_users` 检查。T-00047 在 TDS 中明确：
- **MUST**：REST 路径补齐 `filter_content(content)` 后再写 DB + 广播，确保 `payload.content` 与 WS 路径等价。
- **MAY（不在 T-00047 范围）**：muted 检查 / msg_id 幂等去重在 REST 路径上由运营调用方自行约束（HTTP `Idempotency-Key` 头由后续 Task 接入）。

### 6.8.4 客户端实现锁定（PROTO-1，T-00047 / T-30054）

| 端 | 文件 | 调用入口 | grep-able 字符串断言 |
|----|------|---------|---------------------|
| Android | `app/android/app/src/main/java/com/voice/room/android/feature/room/RoomViewModel.kt::sendMessage` | `wsClient.sendEnvelope(type = "SendMessage", ...)` | 集成测试断言发送 JSON 字节序列包含 `"type":"SendMessage"` |
| Android | 同上 | **不**得调用 Retrofit `POST /chat-messages` | mockWebServer 期望该路径 0 次调用 |
