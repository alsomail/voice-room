> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-AUTH API - 用户认证 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium/firefox/webkit, workers=1)  
**关联任务**: T-0000S (redis-cli 容器化)

## 测试结果（3 浏览器合计）

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-AUTH-00001 | 发送验证码 - 合法手机号首次成功 | ✅ PASS ×3 |
| TC-AUTH-00002 | 验证码 60s 冷却 42901 | ✅ PASS ×3 |
| TC-AUTH-00003 | 每日限额边界值 Max=10 | ✅ PASS ×3 |
| TC-AUTH-00004 | 手机号格式等价类覆盖 | ✅ PASS ×3 |
| TC-AUTH-00005 | 新用户自动注册 & JWT 签发 | ✅ PASS ×3 |
| TC-AUTH-00006 | 验证码错误 5 次锁定 40105 | ✅ PASS ×3 |
| TC-AUTH-00007 | 验证码已过期 40104 | ✅ PASS ×3 |
| TC-AUTH-00008 | JWT 中间件 - 缺失/非法/过期/iss | ✅ PASS ×3 |
| TC-AUTH-00009 | /users/me 响应不含敏感字段 | ✅ PASS ×3 |
| TC-AUTH-00010 | 登录幂等 5 并发仅注册 1 账号 | ✅ PASS ×3 |
| TC-AUTH-00011 | Admin 登录 - 正确凭证签发 7 天 JWT | ✅ PASS ×3 |
| TC-AUTH-00012 | Admin 登录 - 错误凭证/禁用/注入 | ✅ PASS ×3 |
| TC-AUTH-00013 | Admin JWT 中间件 + RBAC 权限矩阵 | ✅ PASS ×3 |

**统计**: 39 PASS / 0 FAIL / 0 SKIP（3 浏览器 × 13 用例）
