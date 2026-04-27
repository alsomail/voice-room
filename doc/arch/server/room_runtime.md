# 房间运行时模块架构

> **关联 Task**：T-00012（进入房间）、T-00013（离开房间）、T-00014（上麦）、T-00015（下麦）、T-00016（文本消息 WS 广播）、T-00043（消息持久化 + REST 历史查询）

## 一、模块职责

`src/room/` 模块管理房间的内存运行时状态，处理所有房间内 WebSocket 信令（进/退房、上/下麦、聊天消息），与 `src/ws/` 模块协作完成广播。

## 二、核心组件

### 2.1 房间管理器（`room/manager.rs`）

- **`RoomManager`**：`DashMap<Uuid, Arc<RoomState>>`
- `get_or_create(room_id)` 保证同一 `room_id` 只创建一个 `RoomState` 实例
- 全局单例，挂载在 `AppState.room_manager`

### 2.2 房间状态（`room/state.rs`）

```rust
pub struct RoomState {
    pub room_id: Uuid,
    pub members: DashMap<Uuid, MemberInfo>,           // 在房成员
    pub mic_slots: RwLock<Vec<Option<Uuid>>>,          // 麦位（固定 8 槽）
    pub banned_mics: DashSet<Uuid>,                    // 禁麦用户
    pub muted_users: DashSet<Uuid>,                    // 禁言用户
    pub processed_msg_ids: DashSet<String>,            // 幂等去重
}
```

**关键方法**：

| 方法 | 说明 |
|------|------|
| `add_member(user_id, info)` | 加入成员表 |
| `remove_member(user_id)` | 移除成员 |
| `take_mic_slot(mic_index, user_id) -> Result<(), TakeMicError>` | 原子抢麦（写锁内完成全部检查） |
| `leave_mic_slot(user_id) -> Option<usize>` | 原子下麦，返回释放的麦位索引 |
| `remove_from_mic_slots(user_id)` | 离房时自动下麦（幂等） |

### 2.3 敏感词过滤（`room/filter.rs`）

- `filter_content(text) -> String`：基于占位常量做关键词替换（`***`）
- 为后续接入真实敏感词库预留扩展点

## 三、WS 信令处理流程

### 3.1 进入房间（T-00012）— `handle_join_room`

1. 解析 payload → 2. DB 校验房间存在（active）→ 3. 查询用户信息 → 4. 加入 `RoomState.members` → 5. `registry.set_room_id` 关联连接 → 6. 广播 `UserJoined` → 7. 返回 `RoomSnapshot`

### 3.2 离开房间（T-00013）— `do_leave_room`

1. 从 `members` 移除 → 2. `remove_from_mic_slots` 自动下麦 → 3. 广播 `UserLeft` → 4. `stats.user_leave_room` → 5. `registry.clear_room_id`

- **`do_leave_room`** 复用于主动离房（`LeaveRoom` 信令）和被动断线（`handle_socket` 退出）
- 先暂存 `leave_mic_slot` 结果，`clear_room_id` 后再广播 `MicLeft`

### 3.3 上麦（T-00014）— `handle_take_mic`

1. 解析 payload → 2. 房间存在校验 → 3. 用户在房校验 → 4. `banned_mics` 禁麦检查 → 5. `take_mic_slot` 原子占位 → 6. 广播 `MicTaken` → 7. 返回成功

- `take_mic_slot` 是同步函数（写锁不跨 `await`），`RwLock` 保证并发抢麦原子性

### 3.4 下麦（T-00015）— `handle_leave_mic`

1. 解析 payload → 2. 房间存在校验 → 3. 用户在房校验 → 4. `leave_mic_slot` 原子释放 → 5. 广播 `MicLeft` → 6. 返回成功

- 未在麦上时幂等返回成功

### 3.5 文本消息 — WS 广播 (T-00016) + REST 历史 (T-00043)

#### WS 信令处理 — `handle_send_message`

1. 解析 payload → 2. 内容非空校验 → 3. 长度限制（500 字符）→ 4. 房间存在校验 → 5. `muted_users` 禁言检查 → 6. `processed_msg_ids` 幂等去重 → 7. `filter_content` 敏感词净化 → 7.5 DB 插入 → 8. 广播 `RoomMessage`

**关键流程变更**（T-00043）：
- 步骤 7.5 新增：`handle_send_message` 调用 `chat_repo.insert_message(room_id, user_id, filtered_content)` 获取 DB id (`UUID`)
- 步骤 8：使用 DB id 作 `payload.msg_id`；envelope 顶层 `msg_id` 由 `broadcast_to_room` 独立生成，两者职责分离（见 `doc/protocol/websocket_signals.md` §6.7.5）
- DB 插入失败返回 50000，不广播（保证 DB / 广播状态一致）

#### REST 历史查询 — `GET /api/v1/rooms/:room_id/messages`

- **路由**：`src/modules/chat/routes.rs` 注册 `chat_routes()`
- **鉴权**：JWT 必需
- **参数**：`?limit=50&offset=0`（limit 默认 50/上限 100；offset 默认 0/软上限 100_000）
- **排序**：`created_at DESC, id DESC`
- **响应**：`{ items: [{id, user_id?, nickname?, avatar_url?, content, created_at}], total, limit, offset }`
  - `items[].id` 与 WS `payload.msg_id` 对齐供前端去重
  - `nickname` / `avatar_url` 来自 LEFT JOIN `users` 表
  - `content` 为净化后文本（与 WS 广播一致）
- **错误码**：40003（参数非法）、40400（房间不存在）

## 四、错误枚举

```rust
pub enum TakeMicError {
    SlotOutOfRange,      // 麦位索引越界
    SlotOccupied,        // 麦位已被占用
    UserAlreadyOnMic,    // 用户已在其他麦位
    MicBanned,           // 用户被禁麦
}
```

## 五、测试覆盖

| Task | 测试数 | 关键场景 |
|------|--------|----------|
| T-00012 进入房间 | 11 | DB 错误、房间不存在、用户不存在、成功广播 |
| T-00013 离开房间 | 10 | 自动下麦、被动断线、`UserLeft` 广播、幂等 |
| T-00014 上麦 | 9 | 麦位越界、占用、已在麦上、禁麦、并发抢麦 |
| T-00015 下麦 | 9+ | 未在麦上幂等、成功下麦广播、`do_leave_room` 顺序 |
| T-00016 文本消息 (WS) | 14+ | 空内容、超长、禁言、幂等、敏感词净化 |
| T-00043 消息持久化 + REST | 10+ | 迁移幂等、DB 插入、倒序排序、分页、并发无丢失、REST 鉴权 |
