> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-GIFT API - 礼物 REST 端点 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)  
**关联任务**: T-00044 (礼物 REST 端点)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-GIFT-00001 | 送礼 WS + HTTP 双路成功 (User B) | ⏭️ SKIP-KNOWN | - |
| TC-GIFT-00002 | HTTP POST /gifts/send 成功扣款 | ✅ PASS | ~50ms |
| TC-GIFT-00003 | 余额不足 40402 | ✅ PASS | ~10ms |
| TC-GIFT-00004 | 礼物不存在 40403 | ✅ PASS | ~8ms |
| TC-GIFT-00005 | 幂等键重复 200+record_id 相同 | ✅ PASS | ~15ms |
| TC-GIFT-00006 | 并发送礼事务原子性 | ✅ PASS | ~200ms |
| TC-GIFT-00007 | 礼物排行榜更新 | ✅ PASS | ~30ms |

**统计**: 6 PASS / 0 FAIL / 1 SKIP (× 3 browsers = 18 PASS / 0 FAIL / 3 SKIP)

**跳过原因**: TC-GIFT-00001 需要 `E2E_USER_B_TOKEN`（seed 未生成）。SKIP-KNOWN。
