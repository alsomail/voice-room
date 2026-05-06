# Room 模块与 WS 信令处理

**Last Updated**: 2026-05-07  
**Related Task**: T-00101 (Android WS sealed class 反序列化 + RoomViewModel 27+ 野生分支全量迁移)

## WS 消息反序列化层（T-00101）

### 架构概述

```
WS Frame (JSON, snake_case + payload-nested)
    │
    ▼ OkHttpWebSocketClient.onMessage(raw: String)
    │
    ▼ WsGsonFactory.create().fromJson(raw, WsServerMessage::class.java)
    │   └── WsServerMessageTypeAdapter (RuntimeTypeAdapterFactory, type-based dispatch)
    │
    ▼ WsServerMessage (sealed class，28 个子类型 + Unknown 兜底)
    │   ├── MicTaken { payload: MicTakenPayload }
    │   ├── MicLeft { payload: MicLeftPayload }
    │   ├── UserJoined { payload: UserJoinedPayload }
    │   ├── UserLeft { payload: UserLeftPayload }
    │   ├── UserMuted { payload: UserMutedPayload }
    │   ├── AdminChanged { payload: AdminChangedPayload }
    │   ├── RoomInfoUpdated { payload: RoomInfoUpdatedPayload }
    │   ├── Pong { payload: PongPayload }
    │   ├── RoomMessage { payload: RoomMessagePayload }
    │   ├── ... (20 more)
    │   └── Unknown { type: String }
    │
    ▼ RoomViewModel.handleWsMessage(msg: WsServerMessage)
        │
        ▼ when(msg) {
              is WsServerMessage.MicTaken -> updateMicSlot(msg.payload.mic_index)
              is WsServerMessage.UserJoined -> addUserToRoom(msg.payload)
              is WsServerMessage.UserMuted -> muteUser(msg.payload)
              ... (全部 28 种 + Unknown 分支)
          }
```

### 核心文件

| 文件 | 用途 | 关键组件 |
|-----|------|---------|
| `core/ws/model/WsServerMessage.kt` | S→C 信令 sealed class | 28 个子类型 + `Unknown` 兜底，每个均包含 `payload: PayloadType` 嵌套对象 |
| `core/ws/model/WsClientMessage.kt` | C→S 信令 sealed class | `Ping`, `JoinRoom`, `SendMessage`, `TakeMic`, `LeaveMic`, `LeaveRoom` 等 |
| `core/ws/model/payload/*.kt` | Payload 数据类 | 各信令的 `payload` 字段定义，使用 `@SerializedName("snake_case") val camelCase` 对齐 server 协议 |
| `core/ws/model/WsGsonFactory.kt` | Gson 工厂 | 注册 RuntimeTypeAdapterFactory，基于 `type` 字段进行子类型分发 |
| `core/ws/OkHttpWebSocketClient.kt` | WebSocket 客户端 | `onMessage(text: String)` 调用 Gson 解析为 `WsServerMessage` |
| `feature/room/RoomViewModel.kt` | 房间视图模型 | `handleWsMessage(msg: WsServerMessage)` 处理 sealed class dispatch |
| `test/.../WsServerMessageTest.kt` | S→C 测试 | PROTO-1~6 验收，fixture 由 `doc/protocol/schemas/ws/*.schema.json` 校验 |
| `test/.../RoomViewModelWsTest.kt` | 集成测试 | REGRESSION：`handleWsMessage` 内零 bare `?: return` |

### 设计原则

1. **字段名强制 snake_case**  
   所有字段在 JSON 传输层均为 `snake_case`，对应 Kotlin 属性名使用 `@SerializedName` 注解映射为 `camelCase`。

2. **Payload 强制嵌套**  
   S→C 信令格式统一为：
   ```json
   {
     "type": "SignalName",
     "msg_id": "uuid-v4",
     "payload": { "field1": "...", "field2": "..." },
     "timestamp": 1713312000000
   }
   ```
   业务数据始终放在 `payload` 字段，禁止顶层平铺。

3. **未知信令不抛异常**  
   - 接收到未知的 `type` 值时，落入 `WsServerMessage.Unknown` 子类
   - 打 `Log.e("UnknownWsSignal", "type=$type")` + 上报埋点
   - 不抛异常，允许应用继续运行（forward-compatibility）

4. **PROTO-BINDING 注释必填**  
   每个 `handleWsMessage` 分支均需标注：
   ```kotlin
   is WsServerMessage.MicTaken -> {
       // PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
       val micIndex = msg.payload.micIndex
       // ...
   }
   ```

### 已完成的验收

- **PROTO-1~6** ✅：MicTaken/MicLeft/UserJoined/UserLeft/UserMuted/AdminChanged/RoomInfoUpdated 从 server 真实 envelope 解析成功
- **SCHEMA-1** ✅：28 份 fixture 均通过 JSON Schema 校验
- **REGRESSION** ✅：`handleWsMessage` 内 zero bare `?: return` 无日志
- **762 tests** ✅：含新增 4 个 WS 单元测试，3 个 BuildConfigFlavor 预存失败与本 task 无关

### 已知限制与后续工作

1. **WsClientMessage.kt 中 @SerializedName 注解实际无效**  
   因 C→S 路径走 `WsEnvelope.build()` helper 绕过 Gson 序列化，注解当前仅作文档用途。  
   **后续处理**：T-00106 codegen 阶段统一自动化。

2. **forced_by 字段字段状态**  
   `MicTakenPayload` 包含 `forced_by` 字段，但 `doc/protocol/schemas/ws/MicTaken.schema.json` 未定义此字段。  
   **后续处理**：待 server 侧确认后更新 schema。

3. **RoomViewModel 行数增长**  
   当前文件 ~1410 行，已超过推荐 1000 行阈值。  
   **后续处理**：建议独立 Task 拆出 `WsMessageHandler` 类专责信令分发。

## 🔌 协议入口索引

| 方向 | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | Payload 字段 | protocol/ 锚点 |
|------|---------|-----------|------------|--------------|------------|---|
| C→S | WS | `LeaveMic` ⭐ | `RoomViewModel.leaveMic(slotIndex)` | `app/server/src/room/handler/mic.rs::handle_leave_mic` | `payload.mic_index`（可选，integer） | [websocket_signals.md §6.5.5](../../protocol/websocket_signals.md) |

> 另见: [doc/protocol/websocket_signals.md §6.5.5 LeaveMic](../../protocol/websocket_signals.md) —— 跨链接回指本文档

## 相关文档

- 协议定义：[doc/protocol/websocket_signals.md](../../protocol/websocket_signals.md)
- Schema 参考：[doc/protocol/schemas/ws/](../../protocol/schemas/ws/)
- Android 架构总索引：[index.md](./index.md)
- TDS 完整报告：[doc/tds/android/T-00101.md](../../tds/android/T-00101.md)
- T-30055 修复报告：[doc/tds/android/T-30055.md](../../tds/android/T-30055.md)
