> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-WS API - WebSocket 网关 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)  
**关联任务**: T-00041 (心跳断开), T-00042 (管理员事件推送)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-WS-00001 | 握手 JWT 正确/错误 | ✅ PASS | ~3ms |
| TC-WS-00002 | 30s 无心跳断开 | ✅ PASS | 32.6s |
| TC-WS-00003 | 断线重连携带 last_msg_id | ✅ PASS | 7ms |
| TC-WS-00004 | 1000 并发连接 | ✅ PASS | 204ms |
| TC-WS-00005 | 管理员封禁事件推送 | ✅ PASS | 361ms |
| TC-WS-00006 | 关闭房间广播 | ⏭️ SKIP-KNOWN | - |
| TC-WS-00007 | 事件处理失败不影响主服务 | ⏭️ SKIP-KNOWN | - |
| TC-WS-00008 | HyperLogLog 在线人数 | ⏭️ SKIP-KNOWN | - |

**统计**: 5 PASS / 0 FAIL / 3 SKIP (× 3 browsers = 15 PASS / 0 FAIL / 9 SKIP)

**跳过原因**: TC-WS-00006/00007/00008 需要 `redis-cli` 工具（PATH 未找到）。SKIP-KNOWN。
