> 当前状态机：负责人 [E2E] | 状态 [⏭️ SKIP-KNOWN] | 修复轮次 [1/5]

# TC-AUTH API - 认证 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-AUTH-00001~00013 | 全部认证用例 | ⏭️ SKIP-KNOWN |

**统计**: 0 PASS / 0 FAIL / 13 SKIP (× 3 browsers = 0 PASS / 0 FAIL / 39 SKIP)

**跳过原因**: TC-AUTH 全套需要 `redis-cli` 工具（操作 JWT 黑名单）及特殊 token 生成。SKIP-KNOWN。
