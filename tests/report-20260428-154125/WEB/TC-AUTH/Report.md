> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [3/5]

# TC-AUTH WEB - 管理员登录 回归报告

**执行时间**: 2026-04-28 15:44 ~ 16:17  
**执行环境**: local (chromium, workers=1, Midscene AI)  
**关联任务**: T-0000P (Midscene env 注入)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-AUTH-00001 | 登录页 UI + 记住用户名 | ✅ PASS | 56.2s |
| TC-AUTH-00002 | 登录失败 - 错误凭证 + 表单校验 | ✅ PASS | 23.6s |
| TC-AUTH-00003 | 路由守卫 - 未登录重定向 | ✅ PASS | 6.5s |
| TC-AUTH-00004 | Token 过期自动退出 | ✅ PASS | 6.9s |
| TC-AUTH-00005 | i18n 默认中文 | ✅ PASS | 7.5s |

**统计**: 5 PASS / 0 FAIL / 0 SKIP (× 3 browsers = 15 PASS / 0 FAIL / 0 SKIP)

**Midscene 报告**: `midscene_run/report/playwright-2026-04-28_15-44-21-7fd11125.html` (+ 4 more)
