> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-RANKING API - 排行榜 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)  
**关联任务**: T-0000O (ranking perf flake known-issue)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-RANKING-00001 | 参数矩阵 @prod-safe | ✅ PASS | 38ms |
| TC-RANKING-00002 | me.rank 未上榜为 null @prod-safe | ✅ PASS | 19ms |
| TC-RANKING-00003 | p95 ≤100ms | ✅ PASS | 127ms |
| TC-RANKING-00004 | 日/周键 归档 | ⏭️ SKIP-KNOWN | - |

**统计**: 3 PASS / 0 FAIL / 1 SKIP (× 3 browsers = 9 PASS / 0 FAIL / 3 SKIP)

**跳过原因**: TC-RANKING-00004 需要 `redis-cli` 工具操作日/周键归档。SKIP-KNOWN。
