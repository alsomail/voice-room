# 8. WebSocket 信令与房间状态管理

## 8.1 单一事实源

Server 是房间与麦位状态的唯一权威。

客户端允许：
- 发送意图，如 `APPLY_SEAT`、`LEAVE_SEAT`、`SEND_GIFT`
- 被动接收权威事件，如 `SEAT_UPDATED`、`ROOM_SNAPSHOT`

客户端严禁：
- 本地直接修改麦位最终状态
- 自行推断礼物扣费是否成功
- 未等待 ACK / Broadcast 就假定上麦成功

## 8.2 信令格式建议

客户端 -> 服务端：

```json
{
  "msg_id": "01HRX9....",
  "event": "APPLY_SEAT",
  "ts": 1719999999999,
  "payload": {
    "room_id": 10001,
    "seat_no": 3
  }
}
```

服务端 -> 客户端：

```json
{
  "msg_id": "01HRXA....",
  "event": "SEAT_UPDATED",
  "room_id": 10001,
  "version": 42,
  "payload": {
    "seat_no": 3,
    "user_id": 9527,
    "status": "occupied"
  }
}
```

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
