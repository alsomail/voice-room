<!-- 
[AI 写入规约]
1. 本文件由 Plan Agent 创建。TDD Agent 仅填写第四节【实现结果】，其余章节只读。Review Agent 仅填写第五节【Reviewer意见】。
2. 创建后必须在 doc/tasks/index.md 对应 Task 的行内补充链接：[TDS](./tds/[$端]/T-xxx.md)
3. 实现完成后，在本文件底部的【实现结果】章节补充实际的代码路径。
-->

# TDS: [任务名称] (Task ID: T-xxx)

## 一、背景与目标
- **关联需求**：（来自 product.md 的哪个 Feature）
- **本 Task 的目标**：用一句话描述这个 Task 要解决什么问题

## 二、方案设计
### 核心数据流
（用文字或 ASCII 图描述数据从哪来、经过哪些层、到哪里去）

### 涉及文件清单
| 文件路径 | 变更类型 | 说明 |
|----------|----------|------|
| `app/server/src/...` | 新增/修改 | xxx |

### 🔌 协议路径绑定表（Plan 必填，TDD/Review/DoD 禁止跳过）

> ⚠️ **铁律**：本 Task 涉及任何跨端通信（HTTP REST、WebSocket 信令、Redis Pub/Sub 等）必须填写本表；不涉及跨端通信请显式写「N/A — 本 Task 仅 X 端内部改造，不动协议」并保留章节。
>
> **填表原则**：
> 1. 列出本 Task 影响或依赖的**所有**协议入口（同一业务的 HTTP / WS 双路径必须都列）。
> 2. 客户端实际**选用**的那条路径必须加 ⭐（如 chat 写消息 Android 走 `WS SendMessage`，则该行加 ⭐）。
> 3. 字段命名、错误码、数据格式必须与 `doc/protocol/` 锚点完全一致；若 protocol/ 暂无对应章节，**Plan 阶段必须先在 protocol/ 落锚**再回填本表。
> 4. 协议类型枚举：`HTTP REST` / `WS C→S` / `WS S→C` / `WS S→Room 广播` / `WS S→User 单播` / `Redis Pub/Sub`。

| # | 协议类型 | 入口 / 信令名 | 客户端调用方（实文件路径 + 函数）| 服务端处理函数（实文件路径 + 函数）| 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|----------------------------------|-----------------------------------|------------|---------------|
| 1 | WS C→S | `SendMessage` ⭐ | `app/android/.../RoomViewModel.kt::sendMessage` | `app/server/src/room/handler/chat.rs::handle_send_message` | 同房间广播 `RoomMessage` | [websocket_signals.md §6.4](../../protocol/websocket_signals.md) |
| 2 | HTTP REST | `POST /api/v1/chat-messages` | （目前无客户端） | `app/server/src/modules/chat/controller.rs::send_chat_message_handler` | 同房间广播 `RoomMessage`（与 #1 等价形态） | [room_api.md](../../protocol/room_api.md) |

### 接口 / 信令定义
（在 `doc/protocol/` 对应子文件中维护字段、错误码、JSON 形态；本节只引用锚点。**严禁**在本节重新定义字段。）

## 三、TDD 验收用例
（直接从 doc/tasks/index.md 复制过来，并在此基础上扩充边界用例）
- [ ] 正向：xxx
- [ ] 异常：断网场景下 xxx
- [ ] 异常：并发场景下 xxx

### 🔴 协议路径绑定的强制验收（每条绑定行至少 1 条）
- [ ] **PROTO-1**：（覆盖绑定表 #1）— 集成/单测断言客户端真实发送字符串包含 `"type":"SendMessage"` 等关键字段；服务端处理函数被实际触发（trace/log 锚点）。
- [ ] **PROTO-2**：（若有双路径）两条路径产生的广播 envelope 除 envelope.msg_id 外**逐字段相等**。

## 四、实现结果（由 TDD Agent 完成后填写）
- **实际修改的文件**：
- **测试覆盖率**：
- **完成时间**：

## 五、Reviewer意见

> 状态枚举：🟢 通过 | 🔴 未通过
> 每轮 Review 追加一条记录，不要覆盖历史。

### Round 1
- **状态**：（🟢 通过 / 🔴 未通过）
- **审查意见**：
- **改进建议**：（仅未通过时填写）

<!-- 如有多轮 Review，按以下格式追加：
### Round N
- **状态**：
- **审查意见**：
- **改进建议**：
-->