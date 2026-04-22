# 测试套件：AUTH 用户认证（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 无（契约见 `doc/protocol/auth_api.md`、`doc/protocol/admin_api.md`）

覆盖 Task：T-00001~T-00005（App Server 手机号登录链路）、T-10001~T-10003（Admin 登录与 RBAC）。

---

## TC-AUTH-00001：发送验证码 - 合法沙特手机号首次成功
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. App Server 运行于 `http://localhost:3000`，SmsProvider 配置为 Mock。
2. Redis 中 `sms:cooldown:+966512345678` 与 `sms:code:+966512345678` 均不存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/auth/verification-codes`，Body `{"phone":"+966512345678"}` | HTTP 200，响应 JSON `code=0`，`data.expires_in=300`，`data.cooldown=60` |
| 2 | `Redis` | 执行 `GET sms:code:+966512345678` | 返回 6 位数字字符串（匹配正则 `^\d{6}$`），TTL 介于 295-300 秒 |
| 3 | `Redis` | 执行 `TTL sms:cooldown:+966512345678` | 介于 55-60 秒 |

**【数据清理】**
- 删除 Redis 键 `sms:code:+966512345678` 与 `sms:cooldown:+966512345678`。

---

## TC-AUTH-00002：发送验证码 - 60 秒冷却期内重复请求返回 42901
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. Redis 中 `sms:cooldown:+966512345678` 存在且 TTL=30 秒。
2. `sms:code:+966512345678` = `111111`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/auth/verification-codes` Body `{"phone":"+966512345678"}` | HTTP 429，响应 `code=42901`，message 包含 `too frequently` |
| 2 | `Redis` | `GET sms:code:+966512345678` | 仍为 `111111`（未被改写） |

**【数据清理】**
- 删除相关 Redis 键。

---

## TC-AUTH-00003：发送验证码 - 每日限额边界值（Max=10, Max+1=11）
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. `sms:daily:+966512345678` = 9，未过 TTL。
2. `sms:cooldown:+966512345678` 不存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 第 10 次 POST verification-codes（Max 边界） | HTTP 200，`code=0` |
| 2 | `Redis` | `GET sms:daily:+966512345678` | 值为 `10` |
| 3 | `AppServer` | 清除 cooldown 后，第 11 次 POST（Max+1） | HTTP 429，`code=42902` |
| 4 | `Redis` | `GET sms:daily:+966512345678` | 仍为 `10`，未继续递增 |

**【数据清理】**
- 删除 Redis 所有相关键。

---

## TC-AUTH-00004：发送验证码 - 手机号格式等价类覆盖
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. App Server 可用，相关 Redis 键已清空。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST Body `{"phone":"12345678"}`（无国家码） | HTTP 400，`code=40001` |
| 2 | `AppServer` | POST Body `{"phone":"+966abc12345"}`（含字母） | HTTP 400，`code=40001` |
| 3 | `AppServer` | POST Body `{"phone":""}` | HTTP 400，`code=40001` 或 `40002` |
| 4 | `AppServer` | POST Body `{}`（缺字段） | HTTP 400，`code=40002` |
| 5 | `AppServer` | POST Body `{"phone":"+9665123456789012345"}`（超 20 位） | HTTP 400，`code=40001` |
| 6 | `AppServer` | POST Body `{"phone":"' OR '1'='1"}`（SQL 注入尝试） | HTTP 400，`code=40001`，DB 无异常 |

**【数据清理】**
- 无。

---

## TC-AUTH-00005：一键登录 - 新用户自动注册 & JWT 签发
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. `users` 表无 `phone='+966500000001'` 的记录。
2. Redis `sms:code:+966500000001` = `123456`，TTL > 60s。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/auth/login` Body `{"phone":"+966500000001","code":"123456"}` | HTTP 200，`data.token` 非空字符串，`data.user.is_new=true`，`data.user.nickname` 匹配正则 `^User_[a-z0-9]{4}$`，`data.user.coin_balance=0` |
| 2 | `DB` | `SELECT id, phone, coin_balance, deleted_at FROM users WHERE phone='+966500000001'` | 返回 1 行，`coin_balance=0`，`deleted_at IS NULL` |
| 3 | `Redis` | `GET sms:code:+966500000001` | Key 不存在（登录成功后被消费） |
| 4 | `AppServer` | 用上一步 token 调用 `GET /api/v1/users/me` | HTTP 200，`data.id` 与登录返回的 user.id 相同 |

**【数据清理】**
- 删除 users 表中对应测试记录。

---

## TC-AUTH-00006：一键登录 - 验证码错误攻击与 5 次尝试锁定
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. Redis `sms:code:+966500000002` = `111111`，TTL=300。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST login Body `{"phone":"+966500000002","code":"222222"}`（首次错误） | HTTP 401，`code=40103` |
| 2 | `Redis` | `GET sms:attempts:+966500000002` | 值为 `1` |
| 3 | `AppServer` | 连续错误 4 次，使累计达到 5 次 | 第 5 次仍返回 `40103` |
| 4 | `AppServer` | 第 6 次错误尝试 | HTTP 401，`code=40105`（max attempts） |
| 5 | `AppServer` | 第 6 次后改用正确码 `111111` | HTTP 401，`code=40105`（已锁定，不放行） |
| 6 | `DB` | `SELECT count(*) FROM users WHERE phone='+966500000002'` | 仍为 0（未创建账号） |

**【数据清理】**
- 删除 Redis 相关键。

---

## TC-AUTH-00007：一键登录 - 验证码已过期
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. Redis 不存在 `sms:code:+966500000003`（已过期或未发送）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST login `{"phone":"+966500000003","code":"123456"}` | HTTP 401，`code=40104` |

**【数据清理】**
- 无。

---

## TC-AUTH-00008：JWT 中间件 - Token 缺失/非法/过期/iss 不匹配
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 预备 `VALID_TOKEN`（有效 C 端 JWT）、`EXPIRED_TOKEN`（过期）、`ADMIN_TOKEN`（iss=voiceroom-admin）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/users/me`，不带 Authorization 头 | HTTP 401，`code=40101` |
| 2 | `AppServer` | GET 带 `Authorization: Bearer abc.def.ghi` | HTTP 401，`code=40101` |
| 3 | `AppServer` | GET 带 `Authorization: Bearer {EXPIRED_TOKEN}` | HTTP 401，`code=40102` |
| 4 | `AppServer` | GET 带 `Authorization: Bearer {ADMIN_TOKEN}` | HTTP 401，`code=40101`（iss 不匹配） |
| 5 | `AppServer` | GET 带 `Authorization: Bearer {VALID_TOKEN}` | HTTP 200，`data` 含正确 user_id |

**【数据清理】**
- 无。

---

## TC-AUTH-00009：获取用户信息 - 响应不包含敏感字段
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. U1 登录得到 `TOKEN_U1`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/users/me` Bearer TOKEN_U1 | HTTP 200，`data` 包含 id/phone/nickname/avatar/coin_balance/vip_level/created_at |
| 2 | `AppServer` | 检查响应 JSON 字符串 | 不包含 `password`、`password_hash`、`deleted_at`、`updated_at` 等字段名 |

**【数据清理】**
- 无。

---

## TC-AUTH-00010：登录幂等 - 同一验证码 5 并发请求仅注册一个账号
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. `users` 表无 `phone='+966500000010'`。
2. Redis `sms:code:+966500000010` = `888888`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 用 `xargs -P 5` 并发发起 5 个相同 login 请求 | 每个响应 HTTP 200，`data.token` 非空 |
| 2 | `DB` | `SELECT count(*) FROM users WHERE phone='+966500000010'` | =1 |
| 3 | `AppServer` | 比较 5 个响应 `data.user.id` | 所有 id 相同 |

**【数据清理】**
- 删除该 users 记录与 Redis 相关键。

---

## TC-AUTH-00011：Admin 登录 - 正确凭证签发 7 天 JWT
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. Admin Server 运行于 `http://localhost:3001`。
2. `admins` 表存在 `username=admin_op, role=operator`，密码 bcrypt 为 `Pass@123`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST `/api/v1/admin/login` Body `{"username":"admin_op","password":"Pass@123"}` | HTTP 200，`data.token` 非空，`data.admin.role="operator"`，`data.expires_in=604800` |
| 2 | `DB` | `SELECT action, ip FROM admin_logs WHERE admin_id=(SELECT id FROM admins WHERE username='admin_op') ORDER BY created_at DESC LIMIT 1` | 最新一条 action=`login`，ip 非空 |
| 3 | `AdminServer` | 解码 JWT payload | 包含 `iss="voiceroom-admin"`，`role="operator"`，`exp-iat=604800` |

**【数据清理】**
- 无。

---

## TC-AUTH-00012：Admin 登录 - 错误凭证 / 禁用账号 / 注入尝试
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. admins 表存在 `admin_op`（status=active）与 `admin_disabled`（status=disabled）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST login `{"username":"admin_op","password":"wrong"}` | HTTP 401，`code=40106` |
| 2 | `AdminServer` | POST login `{"username":"not_exist","password":"x"}` | HTTP 401，`code=40106`（不区分账号/密码错误，防枚举） |
| 3 | `AdminServer` | POST login `{"username":"admin_disabled","password":"Pass@123"}` | HTTP 403，`code=40302` |
| 4 | `AdminServer` | POST login `{"username":"' OR '1'='1","password":"x"}` | HTTP 401，`code=40106`，DB 日志无 SQL 异常 |
| 5 | `AdminServer` | 对同一账号连续 20 次错误尝试 | 首 N 次 401，若启用风控则后续 429；数据库未出现异常 |

**【数据清理】**
- 无。

---

## TC-AUTH-00013：Admin JWT 中间件 + RBAC 权限矩阵
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 预备 `CS_TOKEN`（cs 只读）、`OP_TOKEN`（operator）、`FIN_TOKEN`（finance）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/users` Bearer CS_TOKEN | HTTP 200（只读放行） |
| 2 | `AdminServer` | POST `/api/v1/admin/users/{id}/ban` Bearer CS_TOKEN | HTTP 403，`code=40301` |
| 3 | `AdminServer` | GET `/api/v1/admin/rooms` Bearer FIN_TOKEN | HTTP 403，`code=40301`（无 RoomRead） |
| 4 | `AdminServer` | GET `/api/v1/admin/stats/overview` Bearer FIN_TOKEN | HTTP 200（finance 有 Stats 权限） |
| 5 | `AdminServer` | 任意 admin 接口带 C 端 JWT（iss=voiceroom） | HTTP 401，`code=40101` |
| 6 | `AdminServer` | GET `/api/v1/admin/me` 不带 Authorization | HTTP 401，`code=40101` |

**【数据清理】**
- 无。
