> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-AUTH WEB - 管理员登录 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium, Midscene AI cache=1)  
**耗时**: 1m 18s

## 测试结果（chromium）

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-AUTH-00001 | 登录页 UI + 记住用户名 | ✅ PASS | 39.2s |
| TC-AUTH-00002 | 登录失败 - 错误凭证 + 表单校验 | ✅ PASS | 19.8s |
| TC-AUTH-00003 | 路由守卫 - 未登录重定向 | ✅ PASS | 6.7s |
| TC-AUTH-00004 | Token 过期自动退出 | ✅ PASS | 7.0s |
| TC-AUTH-00005 | i18n 默认中文 | ✅ PASS | 5.6s |

**统计**: 5 PASS / 0 FAIL / 0 SKIP（chromium）

## 说明

所有测试通过 Midscene AI 视觉检测（`@midscene/web/playwright`），缓存命中 (`MIDSCENE_CACHE=1`)。
