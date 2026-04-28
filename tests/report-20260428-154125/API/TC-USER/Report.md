> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-USER API - Admin 用户管理 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-USER-00001 | 列表 - 分页/检索/XSS 安全 | ✅ PASS | 11ms |
| TC-USER-00002 | 详情 - 含钱包/流水/设备 | ✅ PASS | 6ms |
| TC-USER-00003 | 封禁用户 - 临时/永久 + 审计 + WS 踢下线 | ✅ PASS | 85ms |
| TC-USER-00004 | 非法参数 + 重复封禁幂等 | ✅ PASS | 18ms |
| TC-USER-00005 | 解封 - 状态恢复 + 审计 | ✅ PASS | 68ms |

**统计**: 5 PASS / 0 FAIL / 0 SKIP (× 3 browsers = 15 PASS / 0 FAIL / 0 SKIP)
