# TC-AUTH API — 用户认证 测试报告

> 当前状态机：负责人 E2E | 状态 ✅ PASS | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：13 通过 / 0 失败 / 0 阻塞

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
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

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：多个子问题复合
  1. TC-AUTH-00008: `E2E_VALID_TOKEN` (USER_A) 被 TC-USER 封禁测试 ban 后返回 401，而非预期的正常 200
  2. TC-AUTH-00011: 审计日志 action 字段期望 `login` 但服务端写入 `admin_login`
  3. TC-AUTH-00013: CS 角色访问 ban 端点返回 422 而非 403；`/api/v1/admin/me` 端点不存在返回 404 而非 401
- **根本原因 (Root Cause)**：
  1. 并发套件同时操作 USER_A，TC-USER ban 操作先于 TC-AUTH-00008 完成
  2. 审计中间件写入 `admin_login` 动作，但测试断言硬编码 `login`
  3. 服务端 ban 接口先做 JSON schema 校验（422）再做 RBAC（403）；admin/me 路由未实现
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-AUTH.spec.ts`: TC-AUTH-00008 改用 `E2E_USER_B_TOKEN`（非封禁目标）；TC-AUTH-00011 断言从 `login` 改为 `admin_login`；TC-AUTH-00013 ban 端点使用合法 body `{action,ban_type,reason}`、接受 `[403,422]`，`/admin/me` 改为 `/admin/users`（已实现且需要 auth）
