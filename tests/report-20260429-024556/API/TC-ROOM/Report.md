> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [3/5]

# TC-ROOM API - 房间管理 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-0000S (cross-suite isolation fix)

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-ROOM-00001 | 创建/查询/关闭房间 | ✅ PASS ×3 |
| TC-ROOM-00002 | 分页列表 | ✅ PASS ×3 |
| TC-ROOM-00003 | 房间类型 | ✅ PASS ×3 |
| TC-ROOM-00004 | 权限控制 | ✅ PASS ×3 |
| TC-ROOM-00005 | 房间详情 | ✅ PASS ×3 |
| TC-ROOM-00006 | 成员管理 | ✅ PASS ×3 |
| TC-ROOM-00007 | 麦位快照 | ✅ PASS ×3 |
| TC-ROOM-00008 | 管理员查询 | ✅ PASS ×3 |
| TC-ROOM-00009 | 管理员统计 | ✅ PASS ×3 |
| TC-ROOM-00010 | 管理员强制关闭 | ✅ PASS ×3 |
| TC-ROOM-00011 | 管理员详情 | ✅ PASS ×3 |
| TC-ROOM-00012 | 房间状态筛选 | ✅ PASS ×3 |

**统计**: 36 PASS / 0 FAIL / 0 SKIP（3 浏览器 × 12 用例）

## Bug 修复记录

### 🛠️ TDD 修复记录 (Round 3/5)
- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：在全套件运行时，TC-CHAT-00001 和 TC-MIC firefox/webkit 用例因加入房间失败（40400）而失败。TC-ROOM 自身全部通过，但其副作用破坏了其他套件的测试状态。
- **根本原因 (Root Cause)**：TC-ROOM `beforeAll` 关闭了 USER_A 所有活跃房间（包括 seed room_main），`afterAll` 只关闭测试房间未恢复 seed room。由于 Playwright `workers=1 + fullyParallel=true` 的执行顺序（所有 chromium → 所有 firefox → 所有 webkit），chromium TC-ROOM 在 firefox/webkit TC-CHAT/TC-MIC 运行前已关闭 room_main，导致其他 browser 的测试找不到可用房间。
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-ROOM.spec.ts` `afterAll`：在关闭所有测试房间后，额外执行 `UPDATE rooms SET status='active' WHERE id='${seedRoomId}'` 恢复 seed room 为活跃状态，保证 firefox/webkit 的 TC-CHAT/TC-MIC 有房间可用。
