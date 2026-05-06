# 六、WebSocket 信令（字段级冻结 — T-00100）

> **字段级冻结声明**：本文件已从「描述性文档」升级为「机器可读字段级单一事实源」。
> 所有信令字段以本文档为准，对应 JSON Schema 位于 `doc/protocol/schemas/ws/`。
> 修改任何信令字段必须同步更新本文档和对应 schema 文件。
>
> **协议铁律**（详见 [conventions.md §4/§5/§6](./conventions.md)）：
> - §4：所有字段名强制 `snake_case`
> - §5：WS 业务字段必须放 `payload` 嵌套
> - §6：WS envelope 必须包含 `msg_id` + `timestamp` 双 ID

## 6.1 连接建立

```
ws://host/ws?token=<JWT>
```

WebSocket 升级时服务端验证 JWT Token，鉴权失败返回 HTTP 401 后关闭连接。

## 6.2 心跳

- 客户端每 15 秒发送 `Ping`（PascalCase，对齐其他信令）
- 服务端回复 `Pong`（PascalCase）
- 30 秒无心跳自动断开
- 兼容期：服务端同时接受小写 `ping`/`pong`（历史版本兼容）

**另见对侧路径**：
- Ping 发送方：[Android OkHttpWebSocketClient.startHeartbeat](../arch/android/index.md) | [Server 接收入口](../arch/server/index.md)
- Pong 响应方：[Server 构造出口](../arch/server/index.md) | [Android 接收处理](../arch/android/index.md)

## 6.3 消息通用格式（envelope）

```json
{
  "type": "SignalName",
  "msg_id": "uuid-v4",
  "payload": { "...": "..." },
  "timestamp": 1713312000000
}
```

Result/ACK 通用格式：
```json
{
  "type": "XxxResult",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": { "...": "..." },
  "timestamp": 1713312000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 信令类型（PascalCase） |
| `msg_id` | string (UUID v4) | ✅ | 消息唯一 ID；C→S 由客户端生成，S→C 回显或服务端生成 |
| `payload` | object | 条件 | 业务字段容器；无业务字段时可省略或传 `{}` |
| `timestamp` | int64 (ms) | ✅ | 时间戳（毫秒） |
| `code` | int | Result/ACK 必填 | `0` = 成功，其他 = 错误码 |

---

## 6.4 信令全表索引（28 个核心信令）

| # | 信令名 | 方向 | 说明 | Schema |
|---|--------|------|------|--------|
| 1 | `Ping` | C→S | 心跳探活 | [Ping.schema.json](schemas/ws/Ping.schema.json) |
| 2 | `Pong` | S→C | 心跳应答 | [Pong.schema.json](schemas/ws/Pong.schema.json) |
| 3 | `JoinRoom` | C→S | 加入房间 | [JoinRoom.schema.json](schemas/ws/JoinRoom.schema.json) |
| 4 | `JoinRoomResult` | S→C | 加入房间结果 | [JoinRoomResult.schema.json](schemas/ws/JoinRoomResult.schema.json) |
| 5 | `LeaveRoom` | C→S | 离开房间 | [LeaveRoom.schema.json](schemas/ws/LeaveRoom.schema.json) |
| 6 | `LeaveRoomResult` | S→C | 离开房间结果 | [LeaveRoomResult.schema.json](schemas/ws/LeaveRoomResult.schema.json) |
| 7 | `TakeMic` | C→S | 上麦请求 | [TakeMic.schema.json](schemas/ws/TakeMic.schema.json) |
| 8 | `TakeMicResult` | S→C | 上麦结果 | [TakeMicResult.schema.json](schemas/ws/TakeMicResult.schema.json) |
| 9 | `LeaveMic` | C→S | 下麦请求 | [LeaveMic.schema.json](schemas/ws/LeaveMic.schema.json) |
| 10 | `LeaveMicResult` | S→C | 下麦结果 | [LeaveMicResult.schema.json](schemas/ws/LeaveMicResult.schema.json) |
| 11 | `SendMessage` | C→S | 发送文本消息 | [SendMessage.schema.json](schemas/ws/SendMessage.schema.json) |
| 12 | `SendMessageResult` | S→C | 发消息结果 | [SendMessageResult.schema.json](schemas/ws/SendMessageResult.schema.json) |
| 13 | `SendGift` | C→S | 发送礼物 | [SendGift.schema.json](schemas/ws/SendGift.schema.json) |
| 14 | `SendGiftResult` | S→C | 发礼物结果 | [SendGiftResult.schema.json](schemas/ws/SendGiftResult.schema.json) |
| 15 | `ReportEvent` | C→S | 埋点上报 | [ReportEvent.schema.json](schemas/ws/ReportEvent.schema.json) |
| 16 | `EventReportAck` | S→C | 埋点 ACK | [EventReportAck.schema.json](schemas/ws/EventReportAck.schema.json) |
| 17 | `KickUser` | C→S | 踢出用户 | [KickUser.schema.json](schemas/ws/KickUser.schema.json) |
| 18 | `MuteUser` | C→S | 禁麦/禁言 | [MuteUser.schema.json](schemas/ws/MuteUser.schema.json) |
| 19 | `UnmuteUser` | C→S | 解除禁言/禁麦 | [UnmuteUser.schema.json](schemas/ws/UnmuteUser.schema.json) |
| 20 | `TransferAdmin` | C→S | 任命/撤销管理员 | [TransferAdmin.schema.json](schemas/ws/TransferAdmin.schema.json) |
| 21 | `ForceTakeMic` | C→S | 强制上麦 | [ForceTakeMic.schema.json](schemas/ws/ForceTakeMic.schema.json) |
| 22 | `ForceLeaveMic` | C→S | 强制下麦 | [ForceLeaveMic.schema.json](schemas/ws/ForceLeaveMic.schema.json) |
| 23 | `UserJoined` | S→Room | 用户加入广播 | [UserJoined.schema.json](schemas/ws/UserJoined.schema.json) |
| 24 | `UserLeft` | S→Room | 用户离开广播 | [UserLeft.schema.json](schemas/ws/UserLeft.schema.json) |
| 25 | `MicTaken` | S→Room | 麦位被占广播 | [MicTaken.schema.json](schemas/ws/MicTaken.schema.json) |
| 26 | `MicLeft` | S→Room | 麦位空出广播 | [MicLeft.schema.json](schemas/ws/MicLeft.schema.json) |
| 27 | `RoomMessage` | S→Room | 文本消息广播 | [RoomMessage.schema.json](schemas/ws/RoomMessage.schema.json) |
| 28 | `UserMuted` | S→Room | 禁言/禁麦广播 | [UserMuted.schema.json](schemas/ws/UserMuted.schema.json) |

> **附加信令**（已实现，schema 归档）：
> `GiftReceived`（S→Room）、`AdminChanged`（S→Room）、`RoomInfoUpdated`（S→Room）、
> `BalanceUpdated`（S→C）、`UserKicked`（S→C 点对点）、`KickUserResult`、
> `MuteUserResult`、`UnmuteUserResult`、`TransferAdminResult`、`ForceTakeMicResult`、`ForceLeaveMicResult`

---

## 6.5 C→S 信令详细定义

### 6.5.1 Ping（C→S）

**方向**：客户端 → 服务端 | **Schema**：[schemas/ws/Ping.schema.json](schemas/ws/Ping.schema.json)

```json
{
  "type": "Ping",
  "msg_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"Ping"` |
| `msg_id` | string (UUID v4) | ✅ | 客户端生成，Pong 回显此 msg_id |
| `timestamp` | int64 (ms) | ❌ | 客户端时间戳 |

### 6.5.2 JoinRoom（C→S）

**方向**：客户端 → 服务端 | **Schema**：[schemas/ws/JoinRoom.schema.json](schemas/ws/JoinRoom.schema.json)

```json
{
  "type": "JoinRoom",
  "msg_id": "uuid-v4",
  "payload": {
    "room_id": "550e8400-e29b-41d4-a716-446655440001",
    "access_token": null,
    "last_msg_id": null
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"JoinRoom"` |
| `msg_id` | string (UUID v4) | ✅ | 客户端生成 |
| `payload.room_id` | string (UUID) | ✅ | 目标房间 ID |
| `payload.access_token` | string \| null | 条件 | 密码房必填 |
| `payload.last_msg_id` | string \| null | ❌ | 重连续传游标（§6.9），首次进房省略 |
| `timestamp` | int64 (ms) | ❌ | 客户端时间戳 |

**错误码**：`40003` VALIDATION_ERROR | `40101` UNAUTHORIZED | `40104` PASSWORD_REQUIRED |
`40105` TOKEN_EXPIRED | `40400` ROOM_NOT_FOUND | `42911` KICKED_COOLDOWN

### 6.5.3 LeaveRoom（C→S）

**Schema**：[schemas/ws/LeaveRoom.schema.json](schemas/ws/LeaveRoom.schema.json)

```json
{ "type": "LeaveRoom", "msg_id": "uuid-v4", "timestamp": 1720000000000 }
```

### 6.5.4 TakeMic（C→S）

**Schema**：[schemas/ws/TakeMic.schema.json](schemas/ws/TakeMic.schema.json)

```json
{
  "type": "TakeMic",
  "msg_id": "uuid-v4",
  "payload": { "mic_index": 2 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅ | 目标麦位索引，范围 0–8 |

**错误码**：`40002` INVALID_MIC_INDEX | `40301` ALREADY_ON_MIC | `40302` MIC_BANNED |
`40303` SLOT_OCCUPIED | `40306` MIC_MUTED | `40400` NOT_IN_ROOM

### 6.5.5 LeaveMic（C→S）

**Schema**：[schemas/ws/LeaveMic.schema.json](schemas/ws/LeaveMic.schema.json)

```json
{ "type": "LeaveMic", "msg_id": "uuid-v4", "timestamp": 1720000000000 }
```

**错误码**：`40304` NOT_ON_MIC | `40400` NOT_IN_ROOM

### 6.5.6 SendMessage（C→S）

**Schema**：[schemas/ws/SendMessage.schema.json](schemas/ws/SendMessage.schema.json)
**服务端处理**：`app/server/src/room/handler/chat.rs::handle_send_message`

```json
{
  "type": "SendMessage",
  "msg_id": "uuid-v4",
  "payload": { "content": "مرحبا 大家好" },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"SendMessage"` |
| `msg_id` | string (UUID v4) | ✅ | 幂等 key（同 msg_id 重发不二次广播） |
| `payload.content` | string | ✅ | 1–500 Unicode 字符，去前后空白后非空 |
| `timestamp` | int64 (ms) | ❌ | 客户端时间戳 |

**错误码**：`40001` CONTENT_TOO_LONG | `40002` MISSING_PARAMS | `40303` USER_BANNED |
`40305` CHAT_MUTED | `40400` NOT_IN_ROOM | `50000` DB_PERSIST_FAILED

### 6.5.7 SendGift（C→S）

**Schema**：[schemas/ws/SendGift.schema.json](schemas/ws/SendGift.schema.json)

```json
{
  "type": "SendGift",
  "msg_id": "uuid-v4",
  "payload": {
    "room_id": "uuid", "gift_id": "uuid", "receiver_id": "uuid", "count": 1
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.room_id` | string (UUID) | ✅ | 目标房间 ID |
| `payload.gift_id` | string (UUID) | ✅ | 礼物 ID |
| `payload.receiver_id` | string (UUID) | ✅ | 接收者用户 ID |
| `payload.count` | integer [1-9999] | ✅ | 发送数量 |

**错误码**：`40001` INVALID_COUNT | `40002` MISSING_PARAMS | `40290` INSUFFICIENT_BALANCE |
`40400` SENDER_NOT_IN_ROOM | `40402` GIFT_NOT_AVAILABLE | `40403` RECEIVER_UNAVAILABLE

### 6.5.8 ReportEvent（C→S）

**Schema**：[schemas/ws/ReportEvent.schema.json](schemas/ws/ReportEvent.schema.json)

```json
{
  "type": "ReportEvent",
  "msg_id": "uuid-v4",
  "payload": {
    "events": [
      {
        "event_type": "room_join",
        "user_id": "uuid",
        "device_id": "device-abc123",
        "properties": { "room_id": "uuid" },
        "client_ts": 1720000000000
      }
    ]
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.events` | array | ✅ | 事件列表，1–100 条 |
| `payload.events[].event_type` | string | ✅ | 事件类型 |
| `payload.events[].user_id` | string (UUID) | ✅ | 用户 ID（服务端用 JWT 覆盖） |
| `payload.events[].device_id` | string | ✅ | 设备 ID |
| `payload.events[].properties` | object | ❌ | 事件附加属性 |
| `payload.events[].client_ts` | int64 (ms) | ❌ | 客户端时间戳 |

**错误码**：`40002` PARAMETER_MISSING | `40003` VALIDATION_ERROR | `40204` BATCH_TOO_LARGE | `50000` INTERNAL_ERROR

### 6.5.9 KickUser（C→S）

**Schema**：[schemas/ws/KickUser.schema.json](schemas/ws/KickUser.schema.json)
**权限**：房间 owner 或 admin

```json
{
  "type": "KickUser",
  "msg_id": "uuid-v4",
  "payload": { "target_user_id": "uuid", "reason": "违规发言" },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.target_user_id` | string (UUID) | ✅ | 被踢用户 ID |
| `payload.reason` | string | ❌ | 踢人原因（最多 200 字符） |

**错误码**：`40002` MISSING_PARAMS | `40301` PERMISSION_DENIED | `40302` CANNOT_TARGET_OWNER | `40400` TARGET_NOT_IN_ROOM

### 6.5.10 MuteUser（C→S）

**Schema**：[schemas/ws/MuteUser.schema.json](schemas/ws/MuteUser.schema.json)

```json
{
  "type": "MuteUser",
  "msg_id": "uuid-v4",
  "payload": { "target_user_id": "uuid", "mute_type": "chat", "duration_sec": 300 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.target_user_id` | string (UUID) | ✅ | 被禁言/禁麦用户 ID |
| `payload.mute_type` | string | ✅ | `"chat"` 禁言 \| `"mic"` 禁麦 |
| `payload.duration_sec` | integer | ✅ | 时长（秒）；0 = 解除 |

### 6.5.11 UnmuteUser（C→S）

**Schema**：[schemas/ws/UnmuteUser.schema.json](schemas/ws/UnmuteUser.schema.json)

```json
{
  "type": "UnmuteUser",
  "msg_id": "uuid-v4",
  "payload": { "target_user_id": "uuid", "mute_type": "chat" },
  "timestamp": 1720000000000
}
```

### 6.5.12 TransferAdmin（C→S）

**Schema**：[schemas/ws/TransferAdmin.schema.json](schemas/ws/TransferAdmin.schema.json)

```json
{
  "type": "TransferAdmin",
  "msg_id": "uuid-v4",
  "payload": { "target_user_id": "uuid", "action": "assign" },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.target_user_id` | string (UUID) | ✅ | 目标用户 ID |
| `payload.action` | string | ✅ | `"assign"` 任命 \| `"revoke"` 撤销 |

### 6.5.13 ForceTakeMic（C→S）

**Schema**：[schemas/ws/ForceTakeMic.schema.json](schemas/ws/ForceTakeMic.schema.json)
**说明**：强制将目标用户放到指定麦位（成功后广播 `MicTaken`）

```json
{
  "type": "ForceTakeMic",
  "msg_id": "uuid-v4",
  "payload": { "target_user_id": "uuid", "mic_index": 3 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.target_user_id` | string (UUID) | ✅ | 被强制上麦的用户 ID |
| `payload.mic_index` | integer [0-8] | ✅ | 目标麦位索引 |

### 6.5.14 ForceLeaveMic（C→S）

**Schema**：[schemas/ws/ForceLeaveMic.schema.json](schemas/ws/ForceLeaveMic.schema.json)

```json
{
  "type": "ForceLeaveMic",
  "msg_id": "uuid-v4",
  "payload": { "mic_index": 3 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅ | 目标麦位索引 |

**错误码**：`40404` MIC_NOT_FOUND | `40301` PERMISSION_DENIED

---

## 6.6 S→C 信令详细定义（单播回复）

### 6.6.1 Pong（S→C）

**Schema**：[schemas/ws/Pong.schema.json](schemas/ws/Pong.schema.json)

```json
{
  "type": "Pong",
  "msg_id": "uuid-v4（回显 Ping 的 msg_id）",
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定 `"Pong"` |
| `msg_id` | string (UUID v4) | ✅ | 回显客户端 Ping 的 msg_id |
| `timestamp` | int64 (ms) | ✅ | 服务端时间戳 |

### 6.6.2 JoinRoomResult（S→C）

**Schema**：[schemas/ws/JoinRoomResult.schema.json](schemas/ws/JoinRoomResult.schema.json)

```json
{
  "type": "JoinRoomResult",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": {
    "room": {
      "room_id": "uuid",
      "title": "语音房间标题",
      "owner_id": "uuid",
      "member_count": 12,
      "mic_slots": ["uuid-or-null", null, null, null, null, null, null, null, null]
    }
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.room.room_id` | string (UUID) | ✅（成功时） | 房间 ID |
| `payload.room.title` | string | ✅（成功时） | 房间标题 |
| `payload.room.owner_id` | string (UUID) | ✅（成功时） | 房主用户 ID |
| `payload.room.member_count` | integer | ✅（成功时） | 当前在线人数 |
| `payload.room.mic_slots` | array[9] | ✅（成功时） | 麦位列表（UUID string 或 null） |
| `payload.mic_slot` | integer \| null | ❌ | 当前用户所在麦位索引（若已在麦上） |

### 6.6.3 LeaveRoomResult（S→C）

**Schema**：[schemas/ws/LeaveRoomResult.schema.json](schemas/ws/LeaveRoomResult.schema.json)

```json
{ "type": "LeaveRoomResult", "msg_id": "uuid-v4", "code": 0, "timestamp": 1720000000000 }
```

### 6.6.4 TakeMicResult（S→C）

**Schema**：[schemas/ws/TakeMicResult.schema.json](schemas/ws/TakeMicResult.schema.json)

```json
{
  "type": "TakeMicResult",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": { "mic_index": 2 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅（成功时） | 成功占用的麦位索引 |

### 6.6.5 LeaveMicResult（S→C）

**Schema**：[schemas/ws/LeaveMicResult.schema.json](schemas/ws/LeaveMicResult.schema.json)

```json
{
  "type": "LeaveMicResult",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": { "mic_index": 2 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅（成功时） | 释放的麦位索引 |

### 6.6.6 SendMessageResult（S→C）

**Schema**：[schemas/ws/SendMessageResult.schema.json](schemas/ws/SendMessageResult.schema.json)

```json
{
  "type": "SendMessageResult",
  "msg_id": "uuid-v4（回显原始 client msg_id）",
  "code": 0,
  "message": "ok",
  "timestamp": 1720000000000
}
```

### 6.6.7 SendGiftResult（S→C）

**Schema**：[schemas/ws/SendGiftResult.schema.json](schemas/ws/SendGiftResult.schema.json)

```json
{
  "type": "SendGiftResult",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": { "gift_record_id": "uuid", "total_price": 1040 },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.gift_record_id` | string (UUID) | ✅（成功时） | 礼物记录 ID |
| `payload.total_price` | integer | ✅（成功时） | 总扣款钻石数 |

### 6.6.8 EventReportAck（S→C）

**Schema**：[schemas/ws/EventReportAck.schema.json](schemas/ws/EventReportAck.schema.json)

```json
{
  "type": "EventReportAck",
  "msg_id": "uuid-v4",
  "code": 0,
  "payload": { "received": 98, "rejected_indices": [] },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.received` | integer | ✅ | 实际写入条数 |
| `payload.rejected_indices` | array[integer] | ✅ | 被拒绝条目的索引列表 |

---

## 6.7 S→Room 广播信令详细定义

### 6.7.1 UserJoined（S→Room）

**Schema**：[schemas/ws/UserJoined.schema.json](schemas/ws/UserJoined.schema.json)
**触发**：JoinRoom 成功

```json
{
  "type": "UserJoined",
  "msg_id": "server-uuid-v4",
  "payload": {
    "user_id": "uuid",
    "nickname": "Alice",
    "avatar": "https://cdn.example.com/alice.jpg"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.user_id` | string (UUID) | ✅ | 加入用户 ID |
| `payload.nickname` | string | ✅ | 加入用户昵称 |
| `payload.avatar` | string \| null | ✅ | 加入用户头像 URL，无头像为 null |

### 6.7.2 UserLeft（S→Room）

**Schema**：[schemas/ws/UserLeft.schema.json](schemas/ws/UserLeft.schema.json)
**触发**：LeaveRoom 或断线或被踢

```json
{
  "type": "UserLeft",
  "msg_id": "server-uuid-v4",
  "payload": {
    "user_id": "uuid",
    "reason": "left",
    "operator_id": null
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.user_id` | string (UUID) | ✅ | 离开用户 ID |
| `payload.reason` | string | ❌ | `"left"` \| `"kicked_by_admin"` |
| `payload.operator_id` | string (UUID) \| null | ❌ | 踢人操作者 ID |

### 6.7.3 MicTaken（S→Room）

**Schema**：[schemas/ws/MicTaken.schema.json](schemas/ws/MicTaken.schema.json)
**触发**：TakeMic 或 ForceTakeMic 成功

```json
{
  "type": "MicTaken",
  "msg_id": "server-uuid-v4",
  "payload": {
    "mic_index": 2,
    "user_id": "uuid"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅ | 被占用的麦位索引 |
| `payload.user_id` | string (UUID) | ✅ | 上麦用户 ID |

### 6.7.4 MicLeft（S→Room）

**Schema**：[schemas/ws/MicLeft.schema.json](schemas/ws/MicLeft.schema.json)
**触发**：LeaveMic、LeaveRoom 自动下麦、ForceLeaveMic、KickUser

```json
{
  "type": "MicLeft",
  "msg_id": "server-uuid-v4",
  "payload": {
    "mic_index": 2,
    "user_id": "uuid",
    "forced": false
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.mic_index` | integer [0-8] | ✅ | 空出的麦位索引 |
| `payload.user_id` | string (UUID) | ✅ | 下麦用户 ID |
| `payload.forced` | boolean | ✅ | `true` = 被强制下麦（ForceLeaveMic/KickUser） |

### 6.7.5 RoomMessage（S→Room）

**Schema**：[schemas/ws/RoomMessage.schema.json](schemas/ws/RoomMessage.schema.json)
**触发**：SendMessage 成功 或 REST `POST /api/v1/chat-messages` 成功

```json
{
  "type": "RoomMessage",
  "msg_id": "server-uuid-v4",
  "payload": {
    "msg_id": "db-uuid-v4",
    "user_id": "uuid",
    "content": "مرحبا 大家好"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `msg_id`（envelope） | string (UUID v4) | ✅ | 服务端注入；§6.9 续传游标 |
| `payload.msg_id` | string (UUID v4) | ✅ | DB 行主键（`chat_messages.id`），永久标识 |
| `payload.user_id` | string (UUID) | ✅ | 发送者用户 ID |
| `payload.content` | string | ✅ | **敏感词过滤后**的内容 |

### 6.7.6 UserMuted（S→Room）

**Schema**：[schemas/ws/UserMuted.schema.json](schemas/ws/UserMuted.schema.json)
**触发**：MuteUser 或 UnmuteUser 成功

```json
{
  "type": "UserMuted",
  "msg_id": "server-uuid-v4",
  "payload": {
    "room_id": "uuid",
    "target_user_id": "uuid",
    "type": "chat",
    "duration_sec": 300,
    "expires_at": "2026-04-18T12:05:00Z",
    "operator_id": "uuid"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.room_id` | string (UUID) | ✅ | 房间 ID |
| `payload.target_user_id` | string (UUID) | ✅ | 被禁言/禁麦用户 ID |
| `payload.type` | string | ✅ | `"chat"` \| `"mic"` |
| `payload.duration_sec` | integer | ✅ | 禁言时长；`0` = 解除 |
| `payload.expires_at` | string (ISO 8601) \| null | ❌ | 禁言到期时间 |
| `payload.operator_id` | string (UUID) | ✅ | 操作者 ID |

---

## 6.8 附加已实现信令

### 6.8.1 GiftReceived（S→Room）

**触发**：SendGift 成功广播

```json
{
  "type": "GiftReceived",
  "msg_id": "server-uuid-v4",
  "payload": {
    "gift_record_id": "uuid",
    "sender": { "user_id": "uuid", "nickname": "Alice", "avatar": "https://..." },
    "receiver": { "user_id": "uuid", "nickname": "Bob", "avatar": null },
    "gift": { "id": "uuid", "code": "castle_01", "name": "قصر",
              "icon_url": "https://...", "animation_url": "https://...", "effect_level": 4 },
    "count": 1, "total_price": 520
  },
  "timestamp": 1720000000000
}
```

### 6.8.2 AdminChanged（S→Room）

**触发**：TransferAdmin 成功

```json
{
  "type": "AdminChanged",
  "msg_id": "server-uuid-v4",
  "payload": {
    "room_id": "uuid",
    "admin_user_id": "uuid",
    "previous_admin_id": null,
    "operator_id": "uuid"
  },
  "timestamp": 1720000000000
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `payload.admin_user_id` | string (UUID) \| null | ✅ | 新管理员 ID；null = 撤销 |
| `payload.previous_admin_id` | string (UUID) \| null | ✅ | 前任管理员 ID |
| `payload.operator_id` | string (UUID) | ✅ | 操作者 ID |

### 6.8.3 BalanceUpdated（S→C 单播）

见 §6.4 原 T-00018 定义（S→C 单播，不参与房间续传）。

```json
{
  "type": "BalanceUpdated",
  "msg_id": "server-uuid-v4",
  "payload": { "diamond_balance": 4800, "delta": -520, "reason": "gift_send", "ref_id": "uuid" },
  "timestamp": 1720000000000
}
```

### 6.8.4 RoomInfoUpdated（S→Room）

**触发**：`PATCH /api/v1/rooms/:id` 成功
**服务端实现**：`app/server/src/ws/broadcaster.rs::broadcast_room_info_updated`

```json
{
  "type": "RoomInfoUpdated",
  "msg_id": "server-uuid-v4",
  "payload": {
    "room_id": "uuid",
    "title": "新标题",
    "announcement": "最新公告",
    "category": "music",
    "cover_url": "/assets/covers/default.webp",
    "has_password": false
  },
  "timestamp": 1720000000000
}
```

### 6.8.5 UserKicked（S→C 点对点）

**触发**：KickUser 成功（点对点推送给被踢者，不参与续传）

```json
{
  "type": "UserKicked",
  "msg_id": "server-uuid-v4",
  "payload": {
    "room_id": "uuid",
    "reason": "违规发言",
    "cooldown_sec": 600,
    "operator_nickname": "管理员"
  },
  "timestamp": 1720000000000
}
```

---

## 6.9 重连续传（last_msg_id 回放）

### 6.9.1 客户端契约

在 `JoinRoom.payload` 中可选携带 `last_msg_id?: string`：
- **取值**：客户端最近一次成功收到的房间广播 envelope 上的 `msg_id`（服务端 UUID v4）
- **首次进房**：省略该字段或传 `null`

### 6.9.2 envelope.msg_id vs payload.msg_id（双 ID 职责分裂）

| 字段 | 来源 | 用途 |
|------|------|------|
| `msg_id`（envelope 顶层） | 服务端 broadcast_to_room 注入 | §6.9 续传游标 |
| `payload.msg_id` | `chat_messages.id`（DB 行主键） | 业务级稳定标识，与 REST 历史 GET 对齐 |

**重连续传**：`JoinRoom.last_msg_id` 必须传 envelope 顶层 `msg_id`；**不要**传 `payload.msg_id`。

---

## 6.10 Phase 1 (E-07) 礼物经济信令（历史归档）

> 以下内容已整合进 §6.5.7（SendGift）和 §6.6.7（SendGiftResult），保留此节供历史参考。

| 信令 | 方向 | 关联 Task |
|------|------|-----------|
| `BalanceUpdated` | S→C | T-00018 |
| `SendGift` / `SendGiftResult` | C↔S | T-00020 |
| `GiftReceived` | S→房间广播 | T-00020 |

## 6.11 Phase 1 (E-07.5) 埋点上报信令（历史归档）

> 已整合进 §6.5.8（ReportEvent）和 §6.6.8（EventReportAck）。

## 6.12 Phase 1.5 (E-10) 房间治理信令（历史归档）

> 已整合进 §6.5.9–§6.5.14（KickUser/MuteUser/UnmuteUser/TransferAdmin/ForceTakeMic/ForceLeaveMic）。

### §6.12.1 统一错误码（本 Epic 新增）

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

---

## 🔗 另见对侧路径

> **Android 客户端实现**：见 [`doc/arch/android/room.md`](../arch/android/room.md) 中的 WS 消息反序列化层章节（T-00101 sealed class 层落锚）；协议入口索引详见 [`doc/arch/android/index.md`](../arch/android/index.md#-协议入口索引protocol-entry-index)（28 个信令锚点）。
