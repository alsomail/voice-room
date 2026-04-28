> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [3/5]

# TC-MIC API - 麦位管理 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-0000S (E2E_OP_TOKEN 注入), MicLock 三 Bug 修复

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-MIC-00001 | 正常上麦/下麦流程 | ✅ PASS ×3 | ~200ms |
| TC-MIC-00002 | 上麦冲突（槽位已占） | ✅ PASS ×3 | ~180ms |
| TC-MIC-00003 | Mic Muted 管控 | ✅ PASS ×3 | ~250ms |
| TC-MIC-00004 | 多用户并发上麦 | ✅ PASS ×3 | ~300ms |
| TC-MIC-00005 | 下麦后槽位可再用 | ✅ PASS ×3 | ~200ms |
| TC-MIC-00006 | mic_lock TTL 过期自愈 | ⏭️ SKIP ×3 | - |

**统计**: 15 PASS / 0 FAIL / 3 SKIP（3 浏览器 × 5 通过 + 1 跳过）

**跳过原因**: TC-MIC-00006 需 Redis TTL 精确控制（`redis-cli` DEBUG SLEEP 不可用）

## Bug 修复记录

### 🛠️ TDD 修复记录 (Round 1-3/5)
- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-MIC-00001~00005 在前序回归中因 `mic_lock` 未释放导致后续测试连续返回 40303（SlotOccupied/lock），测试套件崩溃。
- **根本原因 (Root Cause)**：Rust server `RealMicLock` 在三个路径未调用 `release()`：(1) `handle_leave_mic` 下麦成功后；(2) `do_leave_room`（WS 断开路径）调用 `leave_mic_slot` 返回 `Some(idx)` 后；(3) `handle_take_mic` 在 `take_mic_slot` 失败（AlreadyOnMic/SlotOccupied）时。
- **修复方案 (Solution)**：
  - `app/server/src/room/mic_lock.rs`：新增 `release()` 方法到 `MicLock` trait + `FakeMicLock`/`RealMicLock` 实现（Redis DEL）
  - `app/server/src/room/handler/mic.rs`：Bug1 `handle_leave_mic` 下麦成功后调用 `mic_lock.release()`；Bug3 `handle_take_mic` 失败路径调用 `mic_lock.release()`（通过 `mic_lock_acquired` bool 标志）
  - `app/server/src/room/handler/lifecycle.rs`：Bug2 `do_leave_room` `leave_mic_slot` 返回 `Some(idx)` 时调用 `mic_lock.release()`
  - `app/server/src/ws/connection.rs`：`LeaveMicDeps` 和 `LeaveRoomDeps` 注入 `mic_lock: Some(mic_lock.clone())`
  - `tests/scripts/API/TC-MIC.spec.ts`：完整重写 WS 协议（正确 payload 格式）、robust `clearSlot()`、fail-fast `join()`、正确 slot 常量
