> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-GIFT API - 礼物 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-00044 (礼物 REST 端点)

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-GIFT-00001 | 礼物列表 排序 + 缓存 + Accept-Language | ✅ PASS ×3 |
| TC-GIFT-00002 | SendGift 原子事务 + WS 推送 | ✅ PASS ×3 |
| TC-GIFT-00003 | 余额不足 40290 + 回滚 | ✅ PASS ×3 |
| TC-GIFT-00004 | 接收者离麦/不存在 40403 | ✅ PASS ×3 |
| TC-GIFT-00005 | msg_id 幂等 + 并发不超卖 | ✅ PASS ×3 |
| TC-GIFT-00006 | count 边界 0/1/99/100 | ✅ PASS ×3 |
| TC-GIFT-00007 | Admin 礼物 CRUD + 软删 + 审计 | ✅ PASS ×3 |

**统计**: 21 PASS / 0 FAIL / 0 SKIP（3 浏览器 × 7 用例）
