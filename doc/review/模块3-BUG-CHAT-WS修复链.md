# 全局代码审查报告：BUG-CHAT-WS 修复链批次（T-00045 + T-00046 + T-30051 + T-30052）

> **当前状态机**：负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]

> **批次定位**：E2E Round 14~22 驱动的聊天 WS 全链路修复闭环，覆盖 Server 侧 REST 广播端点 + WS 广播可观测性 + Android 侧 WS 接收链路可观测性 + UI 气泡样式。本批次 4 个 Task 均属模块 3（房间内核心功能 · Chat 子模块），横跨 App Server 与 Android 两端。

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
  - **T-00045**（模块 3 · App Server / Chat）：REST `POST /api/v1/chat-messages` 修复广播闭环（BUG-CHAT-WS-BROADCAST）。JWT 鉴权 → INSERT → `broadcast_to_room` 广播 RoomMessage，与 WS SendMessage 路径对齐。
  - **T-00046**（模块 3 · App Server / Chat）：WS 广播可观测性增强（BUG-CHAT-WS-BROADCAST-SILENT）。`broadcast_to_room_inner` 发送失败打 WARN + 清理 stale connection，广播前后打 INFO 统计日志。
  - **T-30051**（模块 3 · Android / Chat）：Android WS 接收链路可观测性增强（BUG-CHAT-WS-ANDROID-SILENT）。5 个关键节点注入 Log 日志（onMessage / parse / dispatch / rvm / ui）。
  - **T-30052**（模块 3 · Android / Chat）：ChatMessageList `UserMessageItem` 气泡样式修复（BUG-CHAT-WS-BUBBLE）。包裹 `Surface` 容器（圆角 + ChatBubble 令牌 + padding），新增 `testTag("chat_bubble")`。
- **关联 TDS**：
  - [T-00045](../tds/server/T-00045.md)（§五 Round 1 单 Task 🟢 已通过）
  - [T-00046](../tds/server/T-00046.md)（§五 Round 1 单 Task 🟢 已通过）
  - [T-30051](../tds/android/T-30051.md)（§八 Round 1 单 Task 🟢 已通过）
  - [T-30052](../tds/android/T-30052.md)（§五 Round 1 单 Task 🟢 已通过）
- **参考 E2E 实证**：
  - Round 22 报告 `tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md`
    - DELTA_WS=+1, DELTA_BCAST=+2（Server 广播闭环已生效）
    - Android 5 节点日志全 ≥ 18，parse failed=0（T-30051 可观测性覆盖完整）
    - `ui: chatMessages collected size=1`（T-30051 节点 5 有效）
    - Midscene Step 5（气泡可见性）PASS（T-30052 视觉效果已生效）
    - Step 6 长按菜单 FAIL — 属独立未实现功能，不在本批次范围
- **关联 commits**：
  - T-00045: `94f7753` / `c0ee2ed` / `beedc85` / `14fc69a`
  - T-00046: `981bbbe` / `c7dc021` / `f1b9ec8` / `4afa713` / `bd0391a`
  - T-30051: `091415c` / `c2ffedb` / `d18095d` / `1d5ed18`
  - T-30052: `0dbd19f` / `897cca4` / `28ef9d0` / `67bbb65`
- **开始时间**：2026-05-05

---

## 2. 审查关切（架构级）

### 关切 ① — T-00045：REST 广播端点的协议契约与广播失败容忍
- REST 写入路径与 WS SendMessage 路径是否完全对齐（INSERT → `broadcast_to_room` 顺序、envelope 结构、`msg_id` 注入机制）？
- envelope 顶层 `msg_id`（UUID v4，由 broadcaster 注入）vs `payload.msg_id`（DB id）职责是否与协议 §6.7 双 ID 契约一致？
- broadcast 失败容忍：单连接 drop 后 REST 是否仍返回 200/0？`broadcast_to_room` 内部 `let _ = sender.send(...)` 是否正确吞错？
- 房间无内存状态（`room_manager.get_room(room_id)` 返回 None）的降级分支是否安全（不写 `recent_broadcasts`，不影响 replay 正确性）？
- 安全：JWT 不入日志，`content` 参数是否有长度/XSS 校验防线？

### 关切 ② — T-00046：广播可观测性的 stale connection 清理与 PII 保护
- `stale_ids` 收集和 `registry.unregister` 的批量清理是否在锁外执行（避免 DashMap 持锁期间二次写入死锁）？
- 单连接失败不阻断其他：`continue` 语义是否正确（不 `break` / 不 `return`）？
- PII 保护：广播日志是否**完全不打印消息正文**？`INFO` / `WARN` 日志仅包含结构化字段（`room_id` / `conn_id` / `total_connections` / `sent` / `failed`）？
- T-00045 新增的 REST 广播路径是否经由同一 `broadcast_to_room_inner`，从而自动继承 T-00046 的可观测性增强？

### 关切 ③ — T-30051：Android WS 接收链路日志的线程安全与 PII 保护
- 5 个节点的 `android.util.Log.*` 调用是否全部在正确的线程上下文执行（OkHttp 工作线程 vs viewModelScope dispatcher）？
- PII 保护：所有日志是否严格使用 `text.take(80)` 或仅打印 `len=...`，不打印完整消息正文？
- `ws: parse failed` 分支的 `Log.e(..., e)` 是否会打印 exception 的 message 中潜在携带的 JWT 或 content 片段？
- dex strings 校验：`ws: received` / `ws: parse` / `rvm: onWsMessage` / `ui: chatMessages collected` 等 8 条字符串均在 production 路径（非测试路径）？
- 业务逻辑零修改：diff 是否**仅**新增 `Log.*` 行 + import + TAG 常量，没有改动任何 `if/when/state copy` 业务分支？

### 关切 ④ — T-30052：气泡样式的 Compose 主题/可访问性/MenaColors 规约
- `MenaColors.ChatBubble` 令牌是否遵循 `ULong + .toInt()` 双值规约（规避 BUG-ANDROID-001 `Color(ULong)` 重载误读 colorspace ID）？
- `Surface` 包裹是否使用 `MaterialTheme.shapes.medium` 圆角（避免硬编码 dp），颜色是否走 `MenaColors.ChatBubble` 令牌（避免魔法 hex）？
- `testTag("chat_bubble")` 是否正确挂在 `Surface` 的 `Modifier` 链上（非内部 Text）？
- `widthIn(max=280.dp)` 约束是否考虑 RTL 镜像兼容？`Modifier.fillMaxWidth()` vs `widthIn` 层级关系是否正确？
- T-30051 注入的 5 节点日志（特别是节点 5 `ui: chatMessages collected`）是否**完整保留**，未被 T-30052 diff 意外删除？
- androidTest `CB-01/02/03` 是否真实通过 instrumented test（不是跳过 / skip）？

### 关切 ⑤ — 跨 Task 端到端一致性
- T-00045 REST 端点广播的 envelope 结构是否与 T-00046 在 `broadcast_to_room_inner` 中被 WARN/INFO 统计完全等价可观测（DELTA_BCAST 计数是否正确涵盖 REST 路径广播）？
- T-00045 → T-00046 广播链路 + T-30051 接收链路 5 节点可观测性 + T-30052 UI 渲染：E2E Round 22 实证的 DELTA_BCAST=+2 / 节点 5 size=1 / Midscene Step 5 PASS 是否形成完整闭环证据？
- 两个 Server Task 的安全红线：JWT 是否不进日志？`content` 参数验证是否防注入？

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见：**

已逐项核查 Server 侧 (`controller.rs` / `dto.rs` / `routes.rs` / `broadcaster.rs` / `registry.rs` / `chat_rest_broadcast_test.rs`) 与 Android 侧 (`OkHttpWebSocketClient.kt` / `RoomViewModel.kt` / `ChatMessageList.kt` / `MenaColors.kt` / `ChatBubbleTest.kt` / `RoomViewModelLoggingTest.kt`) 实际源码，对照 5 项关切结论如下：

#### 关切 ① — T-00045 REST 广播契约（PASS）
- ✅ **顺序正确**：`controller.rs:112-141` `chat_repo.insert_message().await` 返回 `Ok` 后才执行 `broadcast_to_room` / `broadcast_to_room_no_state`，DB 事务提交先于广播，无脏读窗口。
- ✅ **双 ID 契约**：`payload.msg_id = message_id.to_string()`（DB id），envelope 顶层 `msg_id` 由 `broadcaster.rs:61-65` 在 `broadcast_to_room_inner` 中统一注入 UUID v4，与协议 §6.7 完全对齐。
- ✅ **失败容忍**：`broadcaster.rs:88-103` 单连接 `sender.send()` 失败仅记 WARN + 收集 `stale_ids` + `fail_count++`，不传播错误；REST handler 不感知，永远返回 200/`code=0`。
- ✅ **降级分支**：`controller.rs:132-141` 房间无内存状态时走 `broadcast_to_room_no_state`，`broadcaster.rs:75-77` 跳过 `recent_broadcasts.push`，不污染续传缓冲。
- ✅ **安全**：handler 全程未 `tracing::*` JWT；`content` 走 `chars().count()` Unicode 校验（1..=500），与 WS SendMessage 路径一致；XSS 由前端渲染层负责（架构既定边界），存储层照原样保留是合理的。

#### 关切 ② — T-00046 广播可观测性（PASS）
- ✅ **锁外清理**：`broadcaster.rs:79` `registry.get_connections_in_room` 返回 `Vec<(Uuid, Sender)>`（已 collect，DashMap 迭代器在该行已 drop）；循环结束后第 105-107 行才调用 `registry.unregister`，无 DashMap 持锁期间二次写入风险。
- ✅ **单连接失败不阻断**：`for` 循环使用 `match` 自然进入下一次迭代，无 `break`/`return`，语义正确。
- ✅ **PII 保护**：所有 `tracing::info!` / `warn!` / `debug!` 仅带结构化字段 `room_id` / `connection_id` / `total_connections` / `sent` / `failed`，**完全不含**消息正文。
- ✅ **REST 继承可观测性**：`broadcast_to_room` 与 `broadcast_to_room_no_state` 同走 `broadcast_to_room_inner`（line 35 / 47），REST 路径自动获得 INFO+INFO 双行（broadcast: starting / broadcast: done），与 E2E Round 22 实证 DELTA_BCAST=+2 完全一致。

#### 关切 ③ — T-30051 Android 接收链路日志（PASS，含 1 条 P3）
- ✅ **5 节点定位准确**：节点 1 `OkHttpWebSocketClient.kt:121` `Log.i ws: received`；节点 2 `RoomViewModel.kt:900,904,909,912` `Log.* ws: parse *`；节点 3 `RoomViewModel.kt:926` `Log.d ws: dispatch`；节点 4 `RoomViewModel.kt:885` `Log.i rvm: onWsMessage`；节点 5 `ChatMessageList.kt:60` `Log.d ui: chatMessages collected`。
- ✅ **PII 截断**：所有可能携带正文的位置统一 `text.take(80)`；其余仅打印 `len=` 或 `type=` / `roomId=` 标识，绝无完整正文。
- ✅ **线程安全**：`android.util.Log` 自身线程安全；节点 1 在 OkHttp listener 线程、节点 4/5 在 `viewModelScope` Dispatcher、节点 2/3 在 collect 同步上下文，无共享可变状态被日志触碰。
- ✅ **production 路径**：8 条 dex strings 均位于 `src/main` 树下，未污染测试路径。
- ✅ **零业务改动**：diff 仅新增 `Log.*` 行 + `import android.util.Log` + TAG 常量；未触动任何 `if/when/state.copy()` 业务逻辑（已与 RoomViewModelChatTest 等价回归通过 `RoomViewModelLoggingTest` 守护）。
- 🟡 **P3（建议改进，非阻断）**：`RoomViewModel.kt:904` `Log.e(TAG, "ws: parse failed head=${raw.take(80)}", e)` 将 `Throwable e` 整个传入。Gson `JsonSyntaxException.message` 通常仅含偏移与首个非法 token，不会回放完整正文，但属于"未严格控制 PII 边界"。建议后续仅打印 `e.javaClass.simpleName` + `e.message?.take(120)`。**不阻断当前批次**。

#### 关切 ④ — T-30052 气泡样式（PASS）
- ✅ **MenaColors.ChatBubble**：`MenaColors.kt:26,43` 严格遵守 `CHAT_BUBBLE_VALUE: ULong = 0xFF2A2A2AuL` + `Color(CHAT_BUBBLE_VALUE.toInt())` 双值规约，规避 BUG-ANDROID-001（`Color(ULong)` 误读 colorspace ID）。
- ✅ **主题圆角**：`ChatMessageList.kt:117` 使用 `MaterialTheme.shapes.medium`，无硬编码 dp。
- ✅ **testTag 位置**：`ChatMessageList.kt:114-116` `Surface(modifier = Modifier.widthIn(...).testTag("chat_bubble"))`，挂在 Surface Modifier 链而非内部 Text，CB-02 可正确命中。
- ✅ **RTL 兼容**：`Row` 默认 `Arrangement.Start`，Compose 在 RTL 自动镜像；`widthIn(max=280.dp)` 仅约束尺寸不锁定方向，安全。
- ✅ **节点 5 日志保留**：`ChatMessageList.kt:60` `ui: chatMessages collected size=...` 完整保留，未被 T-30052 diff 删除。
- ✅ **CB-01/02/03 真实通过**：`ChatBubbleTest.kt` 使用 `createAndroidComposeRule<ComponentActivity>()` + `MenaTheme` 真实 Compose 环境，`assertIsDisplayed()` 强断言，无 skip/ignore；E2E Round 22 Midscene Step 5 PASS 提供视觉双重佐证。

#### 关切 ⑤ — 跨 Task 端到端闭环（PASS）
- ✅ **REST 路径完全继承可观测性**：`controller.rs` → `broadcast_to_room*` → `broadcast_to_room_inner` → INFO `broadcast: starting` + INFO `broadcast: done`，DELTA_BCAST=+2 即由这两条日志贡献，与 Round 22 实证完全一致。
- ✅ **完整闭环证据**：DELTA_BCAST=+2（Server 已广播） → 5 节点日志 ≥18 条（Android 接收链全活） → 节点 5 `size=1`（UI 收集器拿到去重后消息） → Midscene Step 5 PASS（气泡视觉渲染），形成 Server→WS→Parse→Dispatch→VM→UI→视觉 的完整证据链。
- ✅ **安全红线**：JWT 仅在 `AuthContext` extractor 中消费，handler/broadcaster 无任何 JWT 进入 `tracing::*`；`content` 长度限制 1..=500 chars 防呆，存储层不做 XSS 转义符合分层规约（前端 Compose `Text` 不解释 HTML，天然免疫）。

---

**审查总结**：
- P0：0 项
- P1：0 项
- P2：0 项
- P3：1 项（T-30051 `Log.e(..., e)` 异常对象建议字段化，**不阻断**）

**本轮结论**: ✅ 审查通过：代码符合架构规范（分层 / PII 保护 / 协议 §6.7 双 ID / 锁顺序），无 P0/P1/P2 缺陷，4 个 Task 端到端形成完整 E2E 实证闭环。
*(已将文档头部状态机修改为：`负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]`)*

**追加 P3 改进建议（不阻断、可在下一个 Chat 相关 Task 中顺便修复）**：
- `app/android/.../RoomViewModel.kt:904`：将 `Log.e(TAG, "ws: parse failed head=${raw.take(80)}", e)` 改为 `Log.e(TAG, "ws: parse failed head=${raw.take(80)} ex=${e.javaClass.simpleName} msg=${e.message?.take(120)}")`，避免 Throwable 完整字段链潜在 PII 泄露。

---
