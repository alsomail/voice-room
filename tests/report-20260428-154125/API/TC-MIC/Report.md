> 当前状态机：负责人 [E2E] | 状态 [⏭️ SKIP-KNOWN] | 修复轮次 [1/5]

# TC-MIC API - 麦位管理 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)  
**关联任务**: T-00042 (Admin 强制断连广播事件 - MIC 操作)

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-MIC-00001 | 上麦空位成功 + 广播 | ⏭️ SKIP-KNOWN |
| TC-MIC-00002 | 麦位被占返回错误 | ⏭️ SKIP-KNOWN |
| TC-MIC-00003 | 禁麦用户无法上麦 | ⏭️ SKIP-KNOWN |
| TC-MIC-00004 | 并发抢同一空位仅一成功 | ⏭️ SKIP-KNOWN |
| TC-MIC-00005 | 仅本人/房主可下麦 | ⏭️ SKIP-KNOWN |
| TC-MIC-00006 | MuteUser / TransferAdmin 房主权限 + 幂等 | ⏭️ SKIP-KNOWN |

**统计**: 0 PASS / 0 FAIL / 6 SKIP (× 3 browsers = 0 PASS / 0 FAIL / 18 SKIP)

**跳过原因**: TC-MIC 全套需要 `E2E_OP_TOKEN`（seed 未生成运营用户 token）。SKIP-KNOWN。
