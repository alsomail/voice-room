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
```json
{ "type":"BalanceUpdated",
  "payload":{ "diamond": 1234, "charm":56, "reason":"gift_send"|"gift_receive"|"admin_adjust" },
  "timestamp": ... }
```

### 6.4.2 SendGift（C→S）
```json
{ "type":"SendGift", "msg_id":"uuid",
  "payload":{ "room_id":"uuid","gift_id":"uuid","receiver_id":"uuid","count":1 } }
```
错误码：`40201` INSUFFICIENT_BALANCE、`40202` RECEIVER_UNAVAILABLE、`40203` GIFT_NOT_AVAILABLE、`40003` 参数非法。

### 6.4.3 GiftReceived（S→房间）
```json
{ "type":"GiftReceived",
  "payload":{ "sender":{...}, "receiver":{...}, "gift":{...}, "count":1,
              "effect_level":1, "total_price": 520 } }
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
