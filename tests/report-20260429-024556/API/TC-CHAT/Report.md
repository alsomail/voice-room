> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [3/5]

# TC-CHAT API - 公屏聊天 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-00043 (聊天消息持久化), T-0000S (USER_B token 注入)

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-CHAT-00001 | SendMessage 正常广播 | ✅ PASS ×3 | ~30ms |
| TC-CHAT-00002 | 内容长度边界 0/1/500/501 | ✅ PASS ×3 | ~30ms |
| TC-CHAT-00003 | 敏感词过滤 / XSS | ✅ PASS ×3 | ~55ms |
| TC-CHAT-00004 | CHAT_MUTED 禁言 | ✅ PASS ×3 | ~155ms |
| TC-CHAT-00005 | msg_id 去重 | ✅ PASS ×3 | ~940ms |

**统计**: 15 PASS / 0 FAIL / 0 SKIP（3 浏览器 × 5 用例）

## Bug 修复记录

### 🛠️ TDD 修复记录 (Round 3/5)
- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-CHAT-00001 在全套件运行时，3 个浏览器均以 `ws recv timeout`（5s）失败。错误发生在 `recvUntil(ws1, JoinRoomResult)` 之后 `recvUntil(ws2, JoinRoomResult)` 监听注册前。
- **根本原因 (Root Cause)**：**WS 消息监听注册时序竞态**。测试在 `ws1.send(JoinRoom)` 和 `ws2.send(JoinRoom)` 后，串行 `await recvUntil(ws1, ...)` 再 `await recvUntil(ws2, ...)`。当服务端处理 ws2 的 JoinRoom 很快时，`JoinRoomResult` 消息在 `recvUntil(ws2, ...)` 注册 `ws.on('message')` 之前就已到达，被 Node.js 丢弃，导致永远等不到。
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-CHAT.spec.ts` TC-CHAT-00001：在两次 `ws.send(JoinRoom)` 之前同时注册两个 Promise（`jr1P = recvUntil(ws1, ...)`, `jr2P = recvUntil(ws2, ...)`），再顺序 `await jr1P; await jr2P`，确保所有 listener 先于消息到达注册完毕。
