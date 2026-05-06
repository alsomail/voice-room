# 全局代码审查报告: 模块10 — Phase 1.7-extended 协议字段全量冻结
> **当前状态机**：负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [0/10]

---

## 0. 流转规则
- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由[GlobalReview]进行全局代码审查
- [GlobalReview]审查通过，则修改负责人 [-] 状态 [✅ Passed]
- [GlobalReview]审查未通过，则修改负责人 [TDD] 状态 [❌ Failed], 并将审查意见填入文档下方
- 处于负责人 [TDD] 状态 [❌ Failed]，则由[TDD]根据审查意见进行代码修复并自测
- [TDD]修复之后，将状态改为负责人 [GlobalReview] 状态 [⏳ In Review]

---

## 1. 审查上下文
- **包含任务**：
  - [T-00100](../tds/infra/T-00100.md) ⭐ — protocol 三层 schema 冻结
  - [T-00101](../tds/android/T-00101.md) — Android sealed class 干掉 27+ 野生反序列化
  - [T-00102](../tds/web/T-00102.md) — Web Zod 运行时字段校验
  - [T-00103](../tds/server/T-00103.md) — Server deny_unknown_fields + schema_guard
  - [T-00104](../tds/infra/T-00104.md) ⭐ — Android×Server 跨语言 E2E 8 场景
  - [T-00105](../tds/adminServer/T-00105.md) — Redis admin:events 双端契约
  - [T-00106](../tds/infra/T-00106.md) — 字段级 AST CI 审计
  - [T-00107](../tds/infra/T-00107.md) — TDS 字段级回填
  - [T-00108](../tds/infra/T-00108.md) — Ping/Pong 三端同步
- **关联模块文档**：[模块9-E2E测试基建 (E2E QA Foundation)](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)
- **开始时间**：2026-05-08

---

## 🔌 协议路径绑定汇总

> 从批次内每个 Task TDS 第二节抽取，作为 GlobalReview P0 必查项输入证据。

### WebSocket 信令绑定

| # | Task | 方向 | 信令名 | 客户端入口 | 服务端处理函数 | Schema 锚点 |
|---|------|------|--------|-----------|--------------|------------|
| 1 | T-00101 | WS S→C | `MicTaken` ⭐ | `RoomViewModel.handleWsMessage::MicTaken` | `app/server/src/room/handler/mic.rs::handle_take_mic` | `schemas/ws/MicTaken.schema.json` |
| 2 | T-00101 | WS S→C | `MicLeft` ⭐ | `RoomViewModel.handleWsMessage::MicLeft` | `app/server/src/room/handler/mic.rs::handle_leave_mic` | `schemas/ws/MicLeft.schema.json` |
| 3 | T-00101 | WS S→C | `UserJoined` ⭐ | `RoomViewModel.handleWsMessage::UserJoined` | `app/server/src/room/handler/lifecycle.rs::handle_join_room` | `schemas/ws/UserJoined.schema.json` |
| 4 | T-00101 | WS S→C | `UserLeft` ⭐ | `RoomViewModel.handleWsMessage::UserLeft` | `lifecycle.rs::handle_leave_room` | `schemas/ws/UserLeft.schema.json` |
| 5 | T-00101 | WS S→C | `UserMuted` ⭐ | `RoomViewModel.handleWsMessage::UserMuted` | `governance/mute.rs::handle_mute_user` | `schemas/ws/UserMuted.schema.json` |
| 6 | T-00101 | WS S→C | `AdminChanged` ⭐ | `RoomViewModel.handleWsMessage::AdminChanged` | `governance/transfer.rs::handle_transfer_admin` | N/A（无独立 schema 文件） |
| 7 | T-00101 | WS S→C | `RoomInfoUpdated` ⭐ | `RoomViewModel.handleWsMessage::RoomInfoUpdated` | (Server 多处) | N/A（无独立 schema 文件） |
| 8 | T-00101/T-00104/T-00108 | WS C→S+S→C | `Ping`/`Pong` ⭐ | `OkHttpWebSocketClient.startHeartbeat` | `app/server/src/ws/connection.rs::handle_envelope::Ping` | `schemas/ws/Ping.schema.json` / `schemas/ws/Pong.schema.json` |
| 9 | T-00104 | WS C→S | `JoinRoom` ⭐ | `RoomViewModel::joinRoom` | `lifecycle.rs::handle_join_room` | `schemas/ws/JoinRoom.schema.json` |
| 10 | T-00104 | WS S→C | `JoinRoomResult` ⭐ | `RoomViewModel::handleWsMessage` | `lifecycle.rs::handle_join_room` | `schemas/ws/JoinRoomResult.schema.json` |
| 11 | T-00104 | WS C→S | `TakeMic` ⭐ | `RoomViewModel::takeMic` | `mic.rs::handle_take_mic` | `schemas/ws/TakeMic.schema.json` |
| 12 | T-00104 | WS S→C | `TakeMicResult` ⭐ | `RoomViewModel::handleWsMessage` | `mic.rs::handle_take_mic` | `schemas/ws/TakeMicResult.schema.json` |
| 13 | T-00104 | WS C→S | `LeaveMic` ⭐ | `RoomViewModel::leaveMic` | `mic.rs::handle_leave_mic` | `schemas/ws/LeaveMic.schema.json` |
| 14 | T-00104 | WS S→C | `LeaveMicResult` ⭐ | `RoomViewModel::handleWsMessage` | `mic.rs::handle_leave_mic` | `schemas/ws/LeaveMicResult.schema.json` |
| 15 | T-00104 | WS C→S | `SendMessage` ⭐ | `RoomViewModel::sendMessage` | `chat.rs::handle_send_message` | `schemas/ws/SendMessage.schema.json` |
| 16 | T-00104 | WS S→Room | `RoomMessage` ⭐ | `RoomViewModel::handleWsMessage` | `chat.rs::handle_send_message` | `schemas/ws/RoomMessage.schema.json` |
| 17 | T-00104 | WS C→S | `SendGift` ⭐ | `GiftPanelViewModel::sendGift` | `gift/send_gift/handler.rs::handle_send_gift` | `schemas/ws/SendGift.schema.json` |
| 18 | T-00104 | WS S→C | `SendGiftResult` ⭐ | `GiftPanelViewModel::handleWsMessage` | `gift/send_gift/handler.rs::handle_send_gift` | `schemas/ws/SendGiftResult.schema.json` |
| 19 | T-00104 | WS C→S | `MuteUser` ⭐ | Admin WS 触发 | `governance/mute.rs::handle_mute` | `schemas/ws/MuteUser.schema.json` |
| 20 | T-00104 | WS C→S | `KickUser` ⭐ | Admin WS 触发 | `governance/kick.rs::handle_kick` | `schemas/ws/KickUser.schema.json` |

### HTTP REST 绑定

| # | Task | 端点 | 客户端调用方 | 服务端处理函数 | Schema 锚点 |
|---|------|------|-------------|--------------|------------|
| 1 | T-00102 | `GET /api/v1/admin/users` ⭐ | `apiClient.adminUsers.list` | `adminServer/src/modules/user/controller.rs::list_users_handler` | `admin_api.md` |
| 2 | T-00102 | `POST /api/v1/admin/users/:id/ban` ⭐ | `apiClient.adminUsers.ban` | `adminServer/src/modules/user/controller.rs::ban_user_handler` | `admin_api.md` |
| 3 | T-00102 | 其余全部 admin endpoint | `apiClient` | `adminServer/src/modules/*/controller.rs` | `admin_api.md` |

### Redis Pub/Sub 绑定

| # | Task | 频道 :: 事件 | 发布端（adminServer） | 消费端（app server） | Schema 锚点 |
|---|------|-------------|---------------------|---------------------|------------|
| 1 | T-00105 | `admin:events :: BanUser` ⭐ | `adminServer/src/modules/user/service.rs::ban_user` | `server/src/events/handler.rs::handle_admin_event` | `schemas/pubsub/BanUser.schema.json` |
| 2 | T-00105 | `admin:events :: UnbanUser` ⭐ | `adminServer/src/modules/user/service.rs::ban_user` | `server/src/events/handler.rs::handle_admin_event` | `schemas/pubsub/UnbanUser.schema.json` |
| 3 | T-00105 | `admin:events :: CloseRoom` ⭐ | `adminServer/src/modules/room/service.rs::force_close_room` | `server/src/events/handler.rs::handle_admin_event` | `schemas/pubsub/CloseRoom.schema.json` |
| 4 | T-00105 | `admin:events :: BroadcastNotice` ⭐ | `adminServer/src/modules/event/notice_service.rs::broadcast_notice` | `server/src/events/handler.rs::handle_admin_event` | `schemas/pubsub/BroadcastNotice.schema.json` |

### N/A 任务（无跨端协议路径）

| Task | 说明 |
|------|------|
| T-00100 | 纯协议层文档与 Schema 落锚，下游任务实施 |
| T-00103 | 服务端内部加固（deny_unknown_fields + schema_guard），无新协议路径 |
| T-00106 | 纯 CI 工具升级（字段级 AST 审计） |
| T-00107 | 纯文档清理（历史 TDS 字段表回填） |

---

## 2. 审查重点清单（P0 必查）

1. **协议契约对齐**：代码字段名必须严格符合 `doc/protocol/schemas/` 中冻结的 JSON Schema（snake_case，payload 嵌套）
2. **PROTO-BINDING 注释**：所有信令处理代码必须有 `// PROTO-BINDING: doc/protocol/schemas/xxx.schema.json` 注释
3. **deny_unknown_fields 完整性**（T-00103）：所有 serde struct 是否都加了属性
4. **sealed class 覆盖完整性**（T-00101）：27+ 信令是否全部覆盖，Unknown 兜底是否正确
5. **Zod schema 字段对齐**（T-00102）：Zod schema 是否与 JSON Schema 完全一致
6. **跨语言 E2E 字段断言**（T-00104）：payload 字段级断言是否到位（注意 D-01~D-04 协议差异）
7. **Redis pub/sub 双端契约**（T-00105）：发布端和消费端 struct 是否完全对齐
8. **CI 审计工具有效性**（T-00106/T-00107）：审计脚本是否能真正发现违规

> **已知风险**：GiftReceived.schema.json 文件缺失（T-00104 D-04），为已知遗留风险，可标注但不阻塞本次审查。

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**

> 审查范围：T-00100 ~ T-00108，完整协议字段冻结批次（Android sealed class / Server schema_guard / Web Zod / AdminServer pub/sub / Ping-Pong 三端同步）。
> 工具依据：逐文件阅读 + `grep -rn "PROTO-BINDING"` / `grep -rn "timestamp()"` / `grep -rn "forced_by"` / `grep -rn "slot_index"` 全仓扫描；对照 `doc/protocol/schemas/ws/*.schema.json` 逐字段比对；阅读真实调用入口与 schema_guard.rs 注册列表。

---

#### 🔴 P0 致命缺陷

- [ ] **缺陷 1**：[级别 P0] **`ForceTakeMic` C→S：server 读取 `slot_index` 字段但冻结 Schema 要求 `mic_index`；同时读取 `room_id` 而 Schema 的 `additionalProperties: false` 不允许该字段**

  - **文件与行号**：
    - `app/server/src/modules/governance/force_mic.rs:110-118`（读取 `room_id`）
    - `app/server/src/modules/governance/force_mic.rs:130-137`（读取 `slot_index`）
    - `doc/protocol/schemas/ws/ForceTakeMic.schema.json:14-19`（payload 限定为 `{target_user_id, mic_index}`，`additionalProperties: false`）
  - **问题说明**：T-00100 冻结后的 schema 规定 `ForceTakeMic` payload 仅含 `target_user_id` + `mic_index`（且 `additionalProperties: false`）。但 server 实现：① 在 `payload.get("slot_index")` 读取字段（字段名与 schema 不符，应为 `mic_index`）——严格按 schema 发送的客户端将永远触发 40002 error；② 在 `payload.get("room_id")` 读取 `room_id`——schema 不允许该字段，客户端若附带 `room_id` 则违反 schema，若不附带则 server 返回 40002。双重死锁：ForceTakeMic 信令在生产环境对任何符合协议规范的客户端**永远失败**。

  - **grep 证据**：
    ```
    # 服务端读取字段名
    $ grep -n "slot_index\|room_id" app/server/src/modules/governance/force_mic.rs
    132:        .and_then(|p| p.get("slot_index"))   ← 应为 "mic_index"
    112:        .and_then(|p| p.get("room_id"))      ← schema 不允许
    
    # Schema 冻结内容
    $ cat doc/protocol/schemas/ws/ForceTakeMic.schema.json
    "required": ["target_user_id", "mic_index"],
    "additionalProperties": false   ← 只允许 target_user_id + mic_index
    ```
  - **修复建议**：① 将 server 读取字段名 `"slot_index"` 改为 `"mic_index"`；② 将 `room_id` 从 payload 移除，改由 WS session context（用户已加入的房间）推断，或将 `room_id` 追加到 schema 的 `properties` 并从 `additionalProperties: false` 放行。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

- [ ] **缺陷 2**：[级别 P0] **`ForceTakeMic` S→C：向 MicTaken 广播中注入 `forced_by` 字段，违反 `MicTaken.schema.json additionalProperties: false`；`schema_guard` 已注册 MicTaken，测试模式下必然 panic**

  - **文件与行号**：
    - `app/server/src/modules/governance/force_mic.rs:186-194`
    - `doc/protocol/schemas/ws/MicTaken.schema.json:14`（`"additionalProperties": false`）
    - `app/server/src/ws/schema_guard.rs:37`（`("MicTaken", ws_schema_str!("MicTaken"))` 已注册验证）
  - **问题说明**：`handle_force_take_mic` 广播的 `MicTaken` envelope 在 `payload` 中包含 `"forced_by": operator_user_id`。但 `MicTaken.schema.json` 的 `payload` 定义为 `additionalProperties: false`，合法字段仅有 `[mic_index, user_id, nickname, avatar]`，`forced_by` 未被定义。`schema_guard.rs` 在 `REGISTERED_SCHEMAS` 中已注册 `MicTaken`；在 `#[cfg(test)]` 测试模式下，任何经过 `guard_outbound_envelope` 的 ForceTakeMic 广播路径**都会 panic**，导致所有关联集成测试崩溃。

  - **grep 证据**：
    ```
    # server 发送 forced_by
    $ grep -n "forced_by" app/server/src/modules/governance/force_mic.rs
    191:            "forced_by": operator_user_id.to_string(),
    
    # schema_guard 注册了 MicTaken
    $ grep -n "MicTaken" app/server/src/ws/schema_guard.rs
    37:        ("MicTaken", ws_schema_str!("MicTaken")),
    
    # Schema 不含 forced_by（payload additionalProperties: false）
    $ cat doc/protocol/schemas/ws/MicTaken.schema.json  → 无 forced_by 属性
    ```
  - **修复建议**：选择其一：① 在 `MicTaken.schema.json` 的 payload properties 中补充 `"forced_by": {"type": ["string", "null"], "format": "uuid"}`（放开 Android `MicTakenPayload.forcedBy` 已有的字段）；② 或将 `forced_by` 移出 `MicTaken` 改为单独的 `ForceTakeMicAck` 信令。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

- [ ] **缺陷 3**：[级别 P0] **`ForceLeaveMic` S→C：向 MicLeft 广播中注入 `forced_by` 字段，违反 `MicLeft.schema.json additionalProperties: false`；`schema_guard` 已注册 MicLeft，测试模式下必然 panic**

  - **文件与行号**：
    - `app/server/src/modules/governance/force_mic.rs:273-282`
    - `doc/protocol/schemas/ws/MicLeft.schema.json:14`（`"additionalProperties": false`，合法字段仅 `[mic_index, user_id, forced]`）
    - `app/server/src/ws/schema_guard.rs:38`（`("MicLeft", ws_schema_str!("MicLeft"))` 已注册）
  - **问题说明**：`handle_force_leave_mic` 广播的 `MicLeft` 包含 `"forced_by": operator_user_id`，而 `MicLeft.schema.json` 的 payload `additionalProperties: false` 仅允许 `{mic_index, user_id, forced}`，`forced_by` 未被定义。后果与缺陷 2 同等严重：测试模式 schema_guard **panic**。

  - **grep 证据**：
    ```
    $ grep -n "forced_by" app/server/src/modules/governance/force_mic.rs
    280:            "forced_by": operator_user_id.to_string(),
    
    $ grep -n "MicLeft" app/server/src/ws/schema_guard.rs
    38:        ("MicLeft", ws_schema_str!("MicLeft")),
    
    $ cat doc/protocol/schemas/ws/MicLeft.schema.json  → 无 forced_by 属性
    ```
  - **修复建议**：在 `MicLeft.schema.json` 的 payload properties 中补充 `"forced_by": {"type": ["string", "null"], "format": "uuid"}`（与 Android `MicLeftPayload.forcedBy` 保持一致）。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

- [ ] **缺陷 4**：[级别 P0] **`AdminChanged` S→C：server 发送 payload 嵌套 snake_case 字段，Android sealed class 期望平铺 camelCase 字段——字段完全错配，Android 端功能性破损**

  - **文件与行号**：
    - `app/server/src/modules/governance/transfer.rs:264-285`（广播端）
    - `app/android/app/src/main/java/com/voice/room/android/core/ws/model/WsServerMessage.kt:289-303`（接收端）
  - **问题说明**：server 广播的 `AdminChanged` 结构为：
    ```json
    {
      "type": "AdminChanged",
      "payload": {
        "room_id": "...",
        "admin_user_id": "...",
        "previous_admin_id": "...",
        "operator_id": "..."
      },
      "timestamp": ...
    }
    ```
    但 Android `WsServerMessage.AdminChanged` 声明为：
    ```kotlin
    data class AdminChanged(
        @SerializedName("userId") val userId: String? = null,  // 期望顶层平铺
        val role: String? = null,
        @SerializedName("msg_id") val msgId: String? = null
    ) : WsServerMessage()
    ```
    Gson 反序列化时将整个 envelope 映射到 `AdminChanged`：`userId`（顶层，不在 payload 中）和 `role`（顶层，server 从未发送）**全部解析为 null**。管理员任命/撤销的 UI 永远收不到有效数据，该功能在 Android 端**完全失效**。T-00101 实现时以「向后兼容平铺字段」注释保留了旧结构，但实际上 server 早已改用 payload 嵌套格式，造成两端长期错配。

  - **grep 证据**：
    ```
    # server 发送端（payload 嵌套，snake_case）
    $ grep -n "admin_user_id\|AdminChanged" app/server/src/modules/governance/transfer.rs
    265:    let admin_changed_envelope = serde_json::json!({
    266:        "type": "AdminChanged",
    267:        "payload": {
    269:            "admin_user_id": target_user_id.to_string(),
    
    # Android 接收端（平铺，camelCase）
    $ grep -n "userId\|AdminChanged" app/android/.../WsServerMessage.kt
    289: data class AdminChanged(
    290:     @SerializedName("userId") val userId: String? = null,
    ```
  - **修复建议**：必须二选一：① 修改 `WsServerMessage.AdminChanged` 为 payload 嵌套结构（`data class AdminChangedPayload(val admin_user_id: String?, val previous_admin_id: String?, val operator_id: String?)`，外层保持 `WsServerMessage` 标准 payload 模式）；② 修改 server `transfer.rs` 改回平铺字段（`userId`/`role`），并补充 `AdminChanged.schema.json` 固化协议——此选项与整体 payload 嵌套架构规范不符，不推荐。同时须补充 `AdminChanged.schema.json` 消除当前「无独立 schema 文件」的 N/A 状态。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

#### 🟠 P1 高危缺陷

- [ ] **缺陷 5**：[级别 P1] **PROTO-BINDING 注释在 server 全部 WS 信令处理器中系统性缺失**

  - **文件与行号**（覆盖所有无注释的广播/响应函数）：
    - `app/server/src/room/handler/mic.rs`（handle_take_mic 广播 MicTaken L140-158；handle_leave_mic 广播 MicLeft L262-272）
    - `app/server/src/room/handler/lifecycle.rs`（broadcast_user_joined L198-208；JoinRoomResult L219-235；UserLeft/LeaveRoomResult 等）
    - `app/server/src/modules/governance/mute.rs`（broadcast_user_muted L492-528）
    - `app/server/src/modules/governance/force_mic.rs`（ForceTakeMic L185-197；ForceLeaveMic L272-285）
    - `app/server/src/modules/governance/transfer.rs`（AdminChanged L264-284）
    - `app/server/src/modules/governance/kick.rs`（UserKicked broadcast）
  - **问题说明**：`doc/review/模块10-Phase1.7-extended-协议字段冻结.md §2 P0 必查项 #2` 明确要求「所有信令处理代码必须有 `// PROTO-BINDING: doc/protocol/schemas/xxx.schema.json` 注释」。全仓 grep 结果显示 server WS handler 文件中**零**命中（仅 `connection.rs` Ping/Pong 和 HTTP DTO 文件有注释）。

  - **grep 证据**：
    ```
    $ grep -rn "PROTO-BINDING" app/server/src/
    # 命中：connection.rs (L27, L31, L83-128)，modules/auth/dto.rs，modules/room/dto.rs
    # 零命中：mic.rs, lifecycle.rs, mute.rs, force_mic.rs, transfer.rs, kick.rs, chat.rs
    ```
  - **修复建议**：在每个信令处理函数的广播代码块顶部添加 `// PROTO-BINDING: doc/protocol/schemas/ws/XxxSignal.schema.json` 注释，覆盖所有 S→C 广播出口及 C→S 解析入口。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

- [ ] **缺陷 6**：[级别 P1] **所有 WS 信令处理器（除 connection.rs 外）使用 `.timestamp()` 输出秒级时间戳，T-00108「三端同步」目标未达成**

  - **文件与行号**：
    - `app/server/src/room/handler/mic.rs` — 6 处 `.timestamp()` (MicTaken L147, TakeMicResult L157, MicLeft L236, LeaveMicResult L246, MicLeft L269, L...)
    - `app/server/src/room/handler/lifecycle.rs` — 6 处 `.timestamp()` (UserJoined L205, JoinRoomResult L232, UserLeft L297, LeaveRoomResult L329 等)
    - `app/server/src/modules/governance/force_mic.rs` — 2 处 `.timestamp()` (L193, L...)
    - `app/server/src/modules/governance/transfer.rs` — 1 处 `.timestamp()` (L273)
  - **问题说明**：T-00108 TDS 目标是「Ping/Pong timestamp 三端 ms 对齐」，仅修复了 `connection.rs` 的 `ping_pong_responses()`（使用 `timestamp_millis()`）。但 T-00100 `conventions.md §6` 记录「待 T-00108 统一改为 ms」，暗示全量修复。若 T-00108 只承诺 Ping/Pong，则 conventions.md 的描述存在误导；若 T-00108 承诺全量，则实现不完整。无论哪种情况，当前状态是 Android/Web 客户端同时解析来自 Ping 的 ms 时间戳和来自 MicTaken/UserJoined 等信令的秒级时间戳，无法建立统一的时序参考。

  - **grep 证据**：
    ```
    $ grep -rn "\.timestamp()" app/server/src/
    mic.rs:147:        "timestamp": chrono::Utc::now().timestamp(),
    mic.rs:157:        "timestamp": chrono::Utc::now().timestamp(),
    lifecycle.rs:205:  "timestamp": chrono::Utc::now().timestamp(),
    force_mic.rs:193:  "timestamp": chrono::Utc::now().timestamp(),
    transfer.rs:273:   "timestamp": chrono::Utc::now().timestamp(),
    # ... 共 15+ 处 .timestamp()（秒）；仅 connection.rs 使用 .timestamp_millis()
    ```
  - **修复建议**：将全部 `chrono::Utc::now().timestamp()` 统一替换为 `chrono::Utc::now().timestamp_millis()`，并更新 `conventions.md §6` 标注此项已完成。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

#### 🟡 P2 一般缺陷

- [ ] **缺陷 7**：[级别 P2] **`schema_guard.rs` 实际注册 12 条 schema，与 T-00103 TDS 声称「覆盖出栈信令 34 条」严重不符；且运行模式为 test-only，非 TDS 描述的 dev-profile**

  - **文件与行号**：`app/server/src/ws/schema_guard.rs:36-49`（REGISTERED_SCHEMAS 仅 12 条）；`app/server/src/ws/schema_guard.rs:56-68`（`guard_outbound_envelope` 在 `#[cfg(test)]` 外为 no-op）
  - **问题说明**：已注册的 12 条 schema 为：`MicTaken / MicLeft / UserJoined / UserLeft / RoomMessage / JoinRoomResult / LeaveRoomResult / TakeMicResult / LeaveMicResult / SendGiftResult / SendMessageResult / Pong`。未覆盖的典型信令（至少 22 条）包括：`UserMuted / UserKicked / AdminChanged / RoomInfoUpdated / KickUserResult / MuteUserResult / UnmuteUserResult / TransferAdminResult / ForceTakeMicResult / ForceLeaveMicResult / GiftReceived / EventReportAck` 等。TDS §1 声明「dev/test profile 启用」，但实现是 `#[cfg(test)]`（仅测试模式），dev 构建中 guard 是空函数。两项描述均与实现不符。
  - **修复建议**：① 补充未覆盖信令的 schema 注册（或明确在 TDS 中记录有意排除的理由）；② 将 `#[cfg(test)]` 改为 `#[cfg(any(test, debug_assertions))]` 以覆盖 dev-profile，或修正 TDS 描述为「仅 test 模式」。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

#### ✅ 通过项（无需修复）

| 检查项 | 结论 | 关键证据 |
|--------|------|---------|
| Android sealed class 覆盖完整性（T-00101）| ✅ 通过 | `WsServerMessage.kt` 28+ 子类，每类有 PROTO-BINDING，Unknown 兜底存在 |
| Web Zod schema 字段对齐（T-00102） | ✅ 通过 | `.passthrough()` 为有意设计（ZOD-4 验收）；`MicSlotSchema` 字段与 HTTP schema 对齐 |
| Redis pub/sub 双端契约（T-00105） | ✅ 通过 | `shared/admin_event.rs` PROTO-BINDING + snake_case enum；publisher 使用严格类型 |
| HTTP DTO `deny_unknown_fields` 完整性（T-00103） | ✅ 通过 | 9 处确认：auth/room/chat/gift/events + IncomingMessage |
| Cross-lang E2E 字段断言（T-00104）| ✅ 通过 | CROSS-3/CROSS-7 含 PROTO-BINDING，字段断言 snake_case，D-01/D-03 差异正确处理 |
| JoinRoomResult mic_slots 格式（T-00101）| ✅ 通过 | WS 用字符串/null 数组；HTTP 用强类型对象；两者各符合对应 schema |
| AdminEvent pub/sub 序列化一致性 | ✅ 通过 | `serde(tag="type", rename_all="snake_case")` 正确；PROTO-BINDING 注释存在 |

---

**本轮结论**: ❌ 存在 P0/P1 级别问题，共 6 项缺陷（P0×4，P1×2，P2×1）。

**P0 缺陷摘要**：
1. `force_mic.rs` 读取 `slot_index`（schema 要求 `mic_index`）+ 读取 `room_id`（schema 不允许）→ ForceTakeMic 对合规客户端**永远返回错误**
2. `force_mic.rs` 广播 `MicTaken.payload.forced_by`（schema `additionalProperties: false` 不含此字段）→ schema_guard **测试模式 panic**
3. `force_mic.rs` 广播 `MicLeft.payload.forced_by`（同上）→ schema_guard **测试模式 panic**
4. `transfer.rs` 广播 `AdminChanged` payload 嵌套 snake_case 字段，Android 期望平铺 camelCase → Android 端**功能性失效**

*(文档头部已修改为：`负责人 [TDD] | 状态 [❌ Failed]`)*

---
