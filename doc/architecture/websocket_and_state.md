# 8. WebSocket 信令与房间状态管理

## 8.1 单一事实源

Server 是房间与麦位状态的唯一权威。

客户端允许：
- 发送意图（如"申请上麦""离麦""送礼"等业务动作；具体 `type` 值见 [`doc/protocol/websocket_signals.md`](../protocol/websocket_signals.md)，**严禁**在本架构文档硬编码）
- 被动接收权威事件（如房间状态增量、房间快照；具体 `type` 值见 protocol/）

客户端严禁：
- 本地直接修改麦位最终状态
- 自行推断礼物扣费是否成功
- 未等待 ACK / Broadcast 就假定上麦成功

## 8.2 信令格式契约位置

> **🔴 唯一契约源**：WS 信令的 envelope 结构、`type` 枚举（`SendMessage` / `RoomMessage` / `SeatApply` / `SeatUpdated` 等）、字段命名、错误码、双 ID 语义（`envelope.msg_id` UUIDv4 唯一推送标识 vs `payload.msg_id` 业务行 id）等**所有字段级定义**，**唯一**事实源在 [`doc/protocol/websocket_signals.md`](../protocol/websocket_signals.md)。
>
> 本节**不再**重复 JSON 形态，避免出现 `event/APPLY_SEAT` 这类与 protocol/ 中 `type/SeatApply` 不一致的"建议格式"导致客户端跑偏。
>
> 本架构文档只描述**语义**：客户端发送意图 → 服务端校验 + 落库 + 状态机推进 → 服务端基于 `RoomStateRepository` 计算增量 → 广播 `RoomState` 增量事件 + 单播必要的响应（如错误）。**严禁**任何端基于本节 8.1 的语义关键词（如 APPLY_SEAT）硬编码字符串发送/匹配；必须以 protocol/ 中的实际 `type` 值为准。
>
> 跨端 Task 的 TDS 必须在第二节维护「协议路径绑定表」（见 [`doc/tds/_template.md`](../tds/_template.md)），把客户端真实调用方与服务端处理函数双向锁定到 protocol/ 锚点。

## 8.3 房间状态同步机制

**同步策略：**
1. 首次入房，下发 `ROOM_SNAPSHOT`，包含 `version`。
2. 每次状态变更广播增量事件，`version` 递增。
3. 断线重连时，客户端携带最近 `version` 尝试回补。
4. UI 只以最新权威版本进行渲染。
5. **乱序丢弃与回补机制必须严格实现：**
   - 若收到的 `version <= 本地版本号`，必须直接丢弃。
   - 若收到的 `version > 本地版本号 + 1`，必须主动请求最新 `ROOM_SNAPSHOT` 强制回补。

## 8.4 RoomStateRepository 抽象

```rust
pub trait RoomStateRepository: Send + Sync {
    async fn get_snapshot(&self, room_id: RoomId) -> Result<RoomSnapshot>;
    async fn apply_seat_change(&self, cmd: SeatChangeCommand) -> Result<RoomStateDelta>;
    async fn add_member(&self, room_id: RoomId, user_id: UserId) -> Result<RoomStateDelta>;
    async fn remove_member(&self, room_id: RoomId, user_id: UserId) -> Result<RoomStateDelta>;
    async fn next_version(&self, room_id: RoomId) -> Result<u64>;
}
```

初期实现：
- 使用 DashMap + RwLock 持有热状态。
- 冷数据仍存 PostgreSQL。
- 热状态包括：在线成员、麦位占用、房主/管理员临时状态、心跳与连接映射、房间版本号。

未来演进：
- 保持接口不变。
- 增加 `RedisRoomStateRepository`。
- 业务层不得直接依赖 DashMap 或 Redis API。

## 8.5 幂等与防重

所有引起状态变化或资金变化的命令必须携带 `msg_id`。

- **去重键**：`user_id + event + msg_id`
- **存储 TTL**：建议 2-10 分钟
- **重复请求**：返回首次结果或错误码 `DUPLICATE_REQUEST`

适用场景：
- 重复上麦
- 重复下麦
- 重复送礼
- 重复踢人
- 弱网重发
