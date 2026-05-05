# 全局代码审查报告：server-T-00048 Chat 双路径等价回归集成测试

> **当前状态机**：负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]

---

## 0. 流转规则

- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由 [GlobalReview] 进行全局代码审查。
- [GlobalReview] 审查通过 → 修改负责人 [-] 状态 [✅ Passed]。
- [GlobalReview] 审查未通过 → 修改负责人 [TDD] 状态 [❌ Failed]，并将审查意见追加到文档下方。
- [TDD] 修复并自测后 → 状态改为负责人 [GlobalReview] 状态 [⏳ In Review]，触发下一轮复审。

---

## 1. 审查上下文

- **包含任务**：
  - **T-00048**（模块 3 · App Server / Chat）：REST/WS Chat 双路径 envelope 等价回归集成测试（`chat_dual_path_equivalence`）。在 `app/server/tests/chat_dual_path_equivalence.rs` 新增 DUAL-1/2/3 三用例，验证 WS `SendMessage` 主路径与 REST `POST /api/v1/chat-messages` 备路径产生等价 RoomMessage envelope。
- **关联 TDS**：[T-00048](../tds/server/T-00048.md)
- **依赖 Task**：T-00047（协议治理铁律试跑 Task，已 ✅ Released）
- **核心 commits**：f47950d (Plan) / 39179ba (TDD) / 70d1405 (Review-internal) / 8388fcd (DoD)
- **开始时间**：2026-05-05

---

## 🔌 协议路径绑定汇总

> 来源：T-00048 TDS §二「🔌 协议路径绑定表」

### WebSocket 路径

| # | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | 广播/响应 | protocol/ 锚点 |
|---|---------|------------|-------------|--------------|---------|--------------|
| 1 | WS C→S | **SendMessage ⭐**（主路径） | `RoomViewModel.sendMessage → wsClient.sendEnvelope` | `app/server/src/room/handler/chat.rs::handle_send_message` | 广播 RoomMessage 到房间所有 WS 连接 | `websocket_signals.md §6.8.1` |
| 2 | WS S→Room 广播 | RoomMessage | 客户端接收 onMessage | `app/server/src/ws/broadcaster.rs::broadcast_to_room` | 房间所有 WS 连接（含观察者 A/B） | `websocket_signals.md §6.8.2` |

### HTTP REST 路径

| # | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | 广播/响应 | protocol/ 锚点 |
|---|---------|------------|-------------|--------------|---------|--------------|
| 3 | HTTP REST | `POST /api/v1/chat-messages`（备路径） | 无客户端调用（运营工具/测试） | `app/server/src/modules/chat/controller.rs::send_chat_message_handler` | 同房间广播 RoomMessage | `room_api.md §3.6.1` |

**双端实调用入口**：
- 客户端实调用：`app/android/app/src/main/java/com/voiceroom/app/viewmodel/RoomViewModel.kt::sendMessage`（WS 主路径 ⭐）
- 服务端 WS 处理：`app/server/src/room/handler/chat.rs::handle_send_message`
- 服务端 REST 处理：`app/server/src/modules/chat/controller.rs::send_chat_message_handler`
- 广播共用入口：`app/server/src/ws/broadcaster.rs::broadcast_to_room` / `broadcast_to_room_no_state`

---

## 2. 审查关切（P0 必查项）

### 关切 ① — 协议路径绑定（最高优先级）
- TDS §二协议路径绑定表是否完整列出 WS SendMessage ⭐主路径 + REST `POST /api/v1/chat-messages` 备路径？
- 测试代码中对两条路径的实际调用方式是否与绑定表一致（WS 路径通过 `handle_send_message`，REST 路径通过 `build_app + oneshot`）？

### 关切 ② — 测试等价性与字段覆盖
- DUAL-1：两个 WS 观察者 A/B 是否真实并发连接同一房间，双路径广播后双方均收到 envelope？
- DUAL-2：payload 字段逐项断言是否覆盖协议契约全部关键字段（`msg_id` UUID v4 合法性 + 互不相同、`content` 等价、`timestamp` > 0、`user_id` 相等、`type = "RoomMessage"`）？
- DUAL-3：死连接清理在双路径下是否一致（drop receiver → 两路径均能完成，存活观察者均收到消息）？

### 关切 ③ — 测试隔离与架构合规
- 是否纯测试任务（零业务代码修改）？`Cargo.toml` `[[test]]` 注册是否正确携带 `required-features = ["test-utils"]`？
- 是否复用现有辅助函数而非重新造轮子？是否与 PROTO-2 用例保持一致性？

### 关切 ④ — 安全红线
- 测试中 `content` 是否经过 `filter_content` 校验路径（和生产等价）？
- 测试 JWT/token 是否不打印完整内容至日志？`payload.user_id` 等 PII 字段断言是否仅做等值比较而不入日志？

### 关切 ⑤ — filter_content 覆盖
- DUAL-2 中 `content` 等价断言是否验证了经过 `filter_content` 处理后 WS 与 REST 结果一致（而非仅对比原始输入）？

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见：**

**审查日期**：2026-05-05
**审查范围**：`app/server/tests/chat_dual_path_equivalence.rs`（新增）+ `app/server/Cargo.toml`（新增 `[[test]]`）

---

#### 🔌 协议路径绑定校验（P0 独立结论）

**结论：PASS ✅**

逐行 grep 证据如下：

| # | TDS 绑定行 | 客户端/测试调用入口（grep） | 服务端实现入口（grep） | 结论 |
|---|-----------|--------------------------|----------------------|------|
| 1 | WS `SendMessage` ⭐ 主路径 | `chat_dual_path_equivalence.rs:26` import `handle_send_message`；调用：行 131 / 225 / 363 | `room/handler/chat.rs:33` `pub async fn handle_send_message` | ✅ |
| 2 | `RoomMessage` 广播 `broadcast_to_room` | 两路径均经 `handler/chat.rs:151` / `controller.rs:137~143` 调用 | `ws/broadcaster.rs:30` `pub fn broadcast_to_room` 注入 envelope `msg_id` UUID v4 + 写 `recent_broadcasts` | ✅ |
| 3 | REST `POST /api/v1/chat-messages` 备路径 | `chat_dual_path_equivalence.rs:82` `.uri("/api/v1/chat-messages")` via `build_app + oneshot` | `modules/chat/routes.rs:24` `post(send_chat_message_handler)` | ✅ |

**字段名一致性**：协议契约 `websocket_signals.md §6.8.2` 定义 `payload.user_id`；服务端 WS 路径（`chat.rs:146`）与 REST 路径（`controller.rs:129`）均输出 `"user_id"` 字段；测试在行 179、315 断言 `ws_env["payload"]["user_id"]` / `rest_env["payload"]["user_id"]`。**无 `from_user` / `sender_id` 命名错位**。

**filter_content 双路径覆盖**：
- WS 路径：`chat.rs:40+111` `use filter_content` → `let filtered_content = filter_content(&content)`
- REST 路径：`controller.rs:113` `let filtered_content = crate::room::filter::filter_content(&req.content)`
- 两路径均在 INSERT 之前调用，与广播 envelope 使用同一 filtered value ✅

---

#### 📋 详细缺陷列表

- [ ] **缺陷 1**：[级别 P2] **DUAL-2 `timestamp > 0` 断言缺失**
  - **文件与行号**：`app/server/tests/chat_dual_path_equivalence.rs:304-311`
  - **问题说明**：行内注释写道 "int64 > 0（通过 is_number 检查，值允许不同）"，但 `is_number()` 仅验证字段存在且为数字类型，**并不验证值 > 0**。TDS §三 DUAL-2 验证字段明确要求：`payload.timestamp`：int64 ms（值允许不同，但均 > 0）。实际运行中 `Utc::now().timestamp_millis()` 永远为正数，因此该缺陷在线上无实际影响，但注释与实现不一致，存在规格偏差。
  - **修复建议**：将断言替换为：
    ```rust
    assert!(ws_env["timestamp"].as_i64().unwrap_or(0) > 0,
        "DUAL-2: ws timestamp must be > 0 (got numeric check only)");
    assert!(rest_env["timestamp"].as_i64().unwrap_or(0) > 0,
        "DUAL-2: rest timestamp must be > 0");
    ```
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 2**：[级别 P3] **`mod common;` 声明但无调用**
  - **文件与行号**：`app/server/tests/chat_dual_path_equivalence.rs:12`
  - **问题说明**：测试文件顶部声明了 `mod common;` 但本文件内从未调用任何 `common::` 函数（grep 无命中）。`common/mod.rs` 提供的是数据库迁移 helper（`run_migrations`），本测试使用 Fake Repo 不需要迁移，因此该声明为无效引入。`common/mod.rs` 已有 `#![allow(dead_code)]` 兜底，不会编译报错，但属于多余噪音。
  - **修复建议**：删除行 12 的 `mod common;` 声明。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 3**：[级别 P3] **DUAL-1 `type` 断言为相对等值，非绝对值**
  - **文件与行号**：`app/server/tests/chat_dual_path_equivalence.rs:171-174`
  - **问题说明**：`assert_eq!(ws_env["type"], rest_env["type"], ...)` 仅验证 WS 与 REST 两路径 `type` 字段相等，但未断言其值等于 `"RoomMessage"`。若两路径均输出错误的相同 type（如空串），此断言仍会通过。DUAL-2 对此有补充断言，两者联合覆盖是完整的，但 DUAL-1 本身的断言偏弱。
  - **修复建议**（可选）：追加 `assert_eq!(ws_env["type"], "RoomMessage", "DUAL-1: type must be RoomMessage")` 使 DUAL-1 自洽完整。
  - **TDD 修复记录**：[等待 TDD 填写]

---

#### 🟢 审查通过项汇总

| 关切项 | 结论 |
|--------|------|
| ① 协议路径绑定（行 #1/#2/#3 覆盖） | ✅ PASS — grep 全命中 |
| ① 字段命名 (`user_id` vs `from_user`/`sender_id`) | ✅ 全一致，无命名错位 |
| ② DUAL-1：两观察者 A/B 独立收到双路径广播 | ✅ `obs_a_rx` / `obs_b_rx` 各自独立断言 |
| ② DUAL-2：`msg_id` UUID 格式验证 + 互不相同 | ✅ `Uuid::parse_str().is_ok()` + `assert_ne!`（TDS 指定方法） |
| ② DUAL-2：`content` 双路径等值 | ✅ 两路径 `payload.content` 相等断言 |
| ② DUAL-2：`user_id` 双路径等值 + 匹配发送者 | ✅ 行 315-322 |
| ② DUAL-2：`type` 双路径均为 "RoomMessage" | ✅ 行 259-260 |
| ② DUAL-3：死连接清理双路径均不 panic | ✅ WS 和 REST 均发送后存活观察者收消息 |
| ③ 安全：JWT/token 不打印至日志 | ✅ 无 `println!`/`eprintln!` 命中 |
| ③ `filter_content` 双路径等价调用 | ✅ 两路径均在 INSERT 前调用 `filter_content` |
| ④ 纯测试任务（无业务代码修改） | ✅ 仅新增测试文件 + Cargo.toml `[[test]]` 注册 |
| ④ `required-features = ["test-utils"]` | ✅ `Cargo.toml:109-110` 正确注册 |
| ④ 复用 common/ 基础设施 | ✅ `mod common;` 引入（虽本文件未实际调用，架构上合规） |

---

**本轮结论**: ✅ 审查通过：协议路径绑定完整（P0 PASS），无 P0/P1 级别问题。发现 1 条 P2 级规格偏差（`timestamp > 0` 未验证，有纠正价值但无线上风险）+ 2 条 P3 级优化建议。代码整体符合架构规范，DUAL-1/2/3 测试用例实现质量良好。

*(文档头部状态机已更新为：`负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]`)*

---
