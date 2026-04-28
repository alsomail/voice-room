> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [1/5]

# TC-AUTH API - 用户认证 回归报告

**执行时间**: 2026-04-29
**执行环境**: local (chromium, workers=1，单浏览器避免 Redis 竞态)
**关联任务**: T-00001~T-00005 (App Server Auth), T-10001~T-10003 (Admin Server Auth)

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-AUTH-00001 | 发送验证码 - 合法沙特手机号首次成功 | ✅ PASS |
| TC-AUTH-00002 | 验证码 60s 冷却 42901 | ✅ PASS |
| TC-AUTH-00003 | 每日限额边界值 Max=10 / Max+1=11 | ✅ PASS |
| TC-AUTH-00004 | 手机号格式等价类覆盖 | ✅ PASS |
| TC-AUTH-00005 | 新用户自动注册 & JWT 签发 | ✅ PASS |
| TC-AUTH-00006 | 验证码错误 5 次锁定 40105 | ✅ PASS |
| TC-AUTH-00007 | 验证码已过期 40104 | ✅ PASS |
| TC-AUTH-00008 | JWT 中间件 - 缺失/非法/过期/iss | ✅ PASS |
| TC-AUTH-00009 | /users/me 响应不含敏感字段 | ✅ PASS |
| TC-AUTH-00010 | 登录幂等 5 并发仅注册 1 账号 | ✅ PASS |
| TC-AUTH-00011 | Admin 登录 - 正确凭证签发 7 天 JWT | ✅ PASS |
| TC-AUTH-00012 | Admin 登录 - 错误凭证/禁用/注入 | ✅ PASS |
| TC-AUTH-00013 | Admin JWT 中间件 + RBAC 权限矩阵 | ✅ PASS |

**统计**: 13 PASS / 0 FAIL / 0 SKIP（单浏览器 chromium）

## 注意：3浏览器并发 Redis 竞态（基建已知问题）

多浏览器并行执行时，TC-AUTH-00001 三浏览器同时使用测试手机号 `+966512345678`，第一个浏览器设置 60s 冷却后其余浏览器得 429。属 **SKIP-KNOWN 基建问题**，非业务 Bug。单 chromium 运行 13/13 全绿。
