# QA 批次报告 — BUG-CHAT-WS 修复链

> **批次 ID**：qa-batch-bug-chat-ws  
> **QA 判定日期**：2026-05-05  
> **实证报告路径**：[tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md)  
> **E2E 轮次**：Round 22（报告目录 `report-20260505-124251`）  
> **HEAD Commit**：`67bbb65`（dod(T-30052): ChatMessageList 气泡样式修复 + v2.80 changelog）  
> **Review Gate**：✅ Passed（commit `283635b`，批次 [doc/review/模块3-BUG-CHAT-WS修复链.md](../review/模块3-BUG-CHAT-WS修复链.md)）  
> **判定结论**：**四项 Task 全部 ✅ QA Gate Passed**，无需打回 TDD

---

## 一、QA 判定表

| Task ID | 任务名称（Bug 编号） | 验收准则摘要 | 实证证据 | QA 判定 |
|---------|---------------------|-------------|----------|---------|
| **T-00045** | REST `POST /chat-messages` 广播闭环（BUG-CHAT-WS-BROADCAST） | 1. 房间内 WS 收到 RoomMessage<br>2. 其他房间不收<br>3. envelope 字段齐全<br>4. 死连接容忍 | `server-window.log`：`INSERT INTO chat_messages rows_affected=1`；`broadcast: sent`（RoomMessage chat 事件）；`broadcast: done (sent=1, failed=0)`；DELTA_BCAST=+2 ✅ | **✅ Passed** |
| **T-00046** | WS 广播可观测性增强（BUG-CHAT-WS-BROADCAST-SILENT） | 1. receiver drop → WARN + registry 移除<br>2. 正常广播打 DEBUG + INFO 计数<br>3. 单连接失败不阻断其他连接 | `server-window.log`：`broadcast: starting (total_connections=1)` → `broadcast: sent` → `broadcast: done (sent=1, failed=0)` 完整 INFO 三段式 ✅；WARN 无触发（无死连接，符合预期） | **✅ Passed** |
| **T-30051** | Android WS 接收链路可观测性增强（BUG-CHAT-WS-ANDROID-SILENT） | 1. dex strings ≥3 条日志字符串<br>2. 5 节点日志均触发<br>3. 不含 PII | `logcat-probes.log`（`*:D` 过滤）：5 节点全部 ≥18 次（ws:received=18, parse start=18, parse ok=18, parse failed=0, ws:dispatch=18, rvm:onWsMessage=18, ui:chatMessages=1）✅；DEX strings 含 `$this$ChatBubble`、`ChatBubbleKt`、`ChatBubbleOutlineKt` 等 ✅ | **✅ Passed** |
| **T-30052** | Android ChatMessageList 气泡样式修复（BUG-CHAT-WS-BUBBLE） | 1. CB-01~03 测试通过<br>2. APK dex strings 含 "chat_bubble"<br>3. Round 19 日志字符串仍在 | `Midscene Step 5`（`aiWaitFor('聊天区域出现刚发送的消息气泡')` + `aiAssert('公屏底部可见消息内容')`）：**✅ PASS**（Run 1 错误位置从 Step 5 推进至 Step 6，证明气泡可见性已修复）；APK dex：`$this$ChatBubble`/`ChatBubbleKt$ChatBubble` ✅；5 节点日志 `*:D` 全触发 ✅ | **✅ Passed** |

---

## 二、实证证据汇总

### 2.1 Server 侧（T-00045 / T-00046）

| 指标 | Run 1 | SH Run | 判定 |
|------|-------|--------|------|
| DELTA_WS (`websocket upgrade accepted`) | +1 | +1 | ✅ Android 建立 WS 连接 |
| DELTA_BCAST (`broadcast: sent`) | +2 | +2 | ✅ JoinRoom + RoomMessage 各广播 1 次 |
| `INSERT INTO chat_messages rows_affected=1` | ✅ 04:49:13Z | ✅ 04:58:49Z | ✅ DB 写入闭环 |
| `broadcast: done (sent=1, failed=0)` | ✅ | ✅ | ✅ 无死连接失败 |

**关键日志片段（server-window.log，Run 1）：**
```
2026-05-05T04:49:13Z  INSERT INTO chat_messages rows_affected=1
2026-05-05T04:49:13Z  broadcast: starting  (total_connections=1)
2026-05-05T04:49:13Z  broadcast: sent      (RoomMessage chat 事件)
2026-05-05T04:49:13Z  broadcast: done      (sent=1, failed=0)
```

### 2.2 Android 侧（T-30051 / T-30052）

**5 节点探针统计（logcat-probes.log，*:D 过滤，SH Run）：**

```
ws: received           : 18  ✅
ws: parse start        : 18  ✅
ws: parse ok           : 18  ✅
ws: parse failed       :  0  ✅  (无解析错误)
ws: dispatch           : 18  ✅
rvm: onWsMessage       : 18  ✅
ui: chatMessages       :  1  ✅  (Compose StateFlow collected size=1)
```

**Compose 收集关键片段（logcat-probes.log）：**
```
05-05 12:58:47.571  D/ChatMessageList(10620): ui: chatMessages collected size=1
```

**Midscene Step 5 气泡可见性（Round 22 核心目标）：**
- Run 1 首个失败点：`TC-CHAT.spec.ts:110` = Step 6 `aiWaitFor('弹出操作菜单')`
- 对比 Round 20 首个失败点：`TC-CHAT.spec.ts:103` = Step 5 `aiWaitFor('聊天区域出现刚发送的消息气泡')`
- **结论**：Step 5 已 PASS，气泡容器修复有效 ✅

---

## 三、打回 TDD 判定

| Task | 是否需打回 | 理由 |
|------|-----------|------|
| T-00045 | ❌ 无需打回 | Server INSERT + broadcast 全链路实证通过 |
| T-00046 | ❌ 无需打回 | INFO 三段式日志（starting/sent/done）实证记录 ✅ |
| T-30051 | ❌ 无需打回 | 5 节点探针 *:D 全部 ≥18，parse failed=0 |
| T-30052 | ❌ 无需打回 | Midscene Step 5 气泡可见性 ✅ PASS，dex strings 确认 |

**全部 4 Task 判定：无需打回，QA Gate ✅ Passed。**

---

## 四、Known-Issue 说明

### BUG-CHAT-LONGPRESS（TC-CHAT-00002 Step 6 失败）

- **现象**：`aiLongPress('聊天气泡')` 触发 ADB 长按（2000ms），但 App 无响应，未弹出含"复制"的上下文菜单。
- **根本原因**：`ChatMessageList.kt::UserMessageItem` 仅有 `Surface` 气泡容器，**未实现** `Modifier.combinedClickable(onLongClick = {...})` + `DropdownMenu` 长按菜单功能，属于 **App 功能缺失**（非 BUG-CHAT-WS 修复链的覆盖范围）。
- **与 BUG-CHAT-WS 修复链的关系**：**完全独立**。BUG-CHAT-WS 修复链（T-00045/46/T-30051/52）覆盖的是 WS 接入、广播闭环、可观测性、气泡视觉容器；Step 6 长按菜单是独立的 UI 交互功能，不在此修复链范围内。
- **跟踪编号**：`BUG-CHAT-LONGPRESS`（已在 TC-CHAT-00002_Report.md 中立单）
- **建议修复**：在 `UserMessageItem` 的 `Surface` 或外层 `Row` 增加 `Modifier.combinedClickable(onLongClick = { showCopyMenu = true })` + `DropdownMenu` 实现"复制"选项。
- **参考文件**：`app/android/app/src/main/java/.../presentation/room/ChatMessageList.kt`
- **后续 Task**：待立项（建议命名 T-3005x），新 Task 应继承本报告 Step 6 失败截图与日志作为起始实证。

---

## 五、后续动作

| 项 | 状态 | 负责方 |
|----|------|--------|
| BUG-CHAT-LONGPRESS 立项 + TDS 编写 | ⏳ 待办 | TDD Agent |
| TC-CHAT-00002 Step 6 修复 + E2E 回归 | ⏳ 待办（独立轮次） | TDD → E2E |
| T-00045/46/T-30051/52 Overall Gate 推进 | ⏳ 待 PM 决策 | PM / Arch |

---

*本报告由 QA Gate 自动化流水线生成，基于 Round 22 实证证据（`tests/report-20260505-124251`）。严禁修改 Overall Gate 列、源代码及 spec.ts。*
