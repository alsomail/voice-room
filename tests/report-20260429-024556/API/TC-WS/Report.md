> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [3/5]

# TC-WS API - WebSocket 网关 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-00041 (心跳断开), T-00042 (管理员事件/关闭房间广播), T-0000S (redis-cli 容器化)

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-WS-00001 | 握手 JWT 正确/错误 | ✅ PASS ×3 | ~5ms |
| TC-WS-00002 | 30s 无心跳断开 | ✅ PASS ×3 | ~32s |
| TC-WS-00003 | 断线重连携带 last_msg_id | ✅ PASS ×3 | ~7ms |
| TC-WS-00004 | 1000 并发连接 | ✅ PASS ×3 | ~195ms |
| TC-WS-00005 | 管理员封禁事件推送 | ✅ PASS ×3 | ~395ms |
| TC-WS-00006 | 关闭房间广播 | ✅ PASS ×3 | ~110ms |
| TC-WS-00007 | 事件处理失败不影响主服务 | ✅ PASS ×3 | ~70ms |
| TC-WS-00008 | HyperLogLog 在线人数 | ✅ PASS ×3 | ~610ms |

**统计**: 24 PASS / 0 FAIL / 0 SKIP（3 浏览器 × 8 用例）

## Bug 修复记录

### 🛠️ TDD 修复记录 (Round 3/5)
- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-WS-00006 `close_room` 广播消息始终在 5s 超时内未到达客户端。前次运行失败留下 MUTED 用户 active 房间导致 POST 返回 409 → `!RID` → 后续运行直接跳过（`⏭️ SKIP`）。
- **根本原因 (Root Cause)**：测试脚本在 line 148 调用 `POST /api/v1/admin/rooms/${RID}/force-close` — **该路由不存在（返回 404）**。正确端点为 `DELETE /api/v1/admin/rooms/{id}`（见 `app/adminServer/src/bootstrap/mod.rs` line 181）。因此 admin 服务从未发布 Redis 事件，app server 从未广播 `close_room`。同时测试缺少 `finally` 清理块，失败后遗留 active 房间阻塞下次运行。
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-WS.spec.ts` TC-WS-00006：
    1. 添加前置清理：`execSync('docker exec vr-postgres psql ...')` 关闭 MUTED 用户所有 active 房间
    2. 将 `POST .../force-close` 改为 `DELETE /api/v1/admin/rooms/${RID}`（方法 + 路径同时修正）
    3. 用 `try/finally` 包裹主体，`finally` 中再次调用 `DELETE` 保证测试后清理
