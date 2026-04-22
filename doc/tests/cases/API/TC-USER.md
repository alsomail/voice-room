# 测试套件：USER 用户管理（Admin API）

> **需求模糊点 (Ambiguity Notes)**：
> - 无（契约见 `doc/protocol/admin_api.md`）

覆盖 Task：T-10007~T-10009（用户列表/详情/封禁/解封）。

---

## TC-USER-00001：Admin 用户列表 - 分页/检索/XSS 安全
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. DB 构造 15 名用户，nickname 含 "阿里" 的 2 人。
2. OP_TOKEN 为 operator 有效 token。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/users?page=1&page_size=10` OP_TOKEN | HTTP 200，items 长度=10，total=15 |
| 2 | `AdminServer` | GET `?keyword=阿里` | items 长度=2 |
| 3 | `AdminServer` | GET `?keyword=<script>alert(1)</script>` | HTTP 200，items=[]，数据库日志无异常 |
| 4 | `AdminServer` | GET `?page=0` | HTTP 400 code=40003 |
| 5 | `AdminServer` | GET `?page_size=101` | HTTP 400 |
| 6 | `AdminServer` | GET 不带 token | 401 |
| 7 | `AdminServer` | GET Bearer FIN_TOKEN（无 UserRead） | 403 code=40301 |

**【数据清理】**
- 清理测试用户。

---

## TC-USER-00002：Admin 用户详情 - 含钱包/流水/设备
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 存在，coin_balance=1234，最近 3 条 wallet_transactions，1 条 device 记录。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/users/{U1}` OP_TOKEN | HTTP 200，`data` 含 profile/wallet/recent_transactions(长度=3)/devices(长度=1) |
| 2 | `AdminServer` | 响应体 | 不含 password_hash/access_token 等敏感字段 |
| 3 | `AdminServer` | GET `/api/v1/admin/users/00000000-0000-0000-0000-000000000000` | 404 code=40400 |
| 4 | `AdminServer` | GET `/api/v1/admin/users/not-uuid` | 400 code=40003 |

**【数据清理】**
- 无。

---

## TC-USER-00003：Admin 封禁用户 - 临时/永久 + 审计 + WS 踢下线
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 在房间 R1 内，WS 活跃。
2. admin_op 执行本操作。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST `/api/v1/admin/users/{U1}/ban` Body `{"type":"temporary","duration_hours":24,"reason":"恶意刷屏"}` OP_TOKEN | HTTP 200 code=0 |
| 2 | `DB` | `SELECT status, ban_until FROM users WHERE id={U1}` | status=`banned`，ban_until ≈ now()+24h（±1 分钟） |
| 3 | `DB` | `admin_logs` 最新 1 条 | action=`user_ban`，target_id=U1，detail 含 reason/duration |
| 4 | `Redis` | MONITOR | PUBLISH `admin:events` payload type=`ban_user` user_id=U1 |
| 5 | `AppServer` | U1 的 WS 连接 | 5s 内先收 BanNotice，后连接关闭 |
| 6 | `AppServer` | R1 内其他成员 WS | 收到 MicLeft + UserLeft 关于 U1 的广播 |
| 7 | `AppServer` | U1 用原 token 调 GET /users/me | 401 或 403 禁止 |

**【数据清理】**
- Unban U1。

---

## TC-USER-00004：Admin 封禁 - 非法参数 + 重复封禁幂等
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 active。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST ban Body `{"type":"invalid"}` OP_TOKEN | 400 code=40003 |
| 2 | `AdminServer` | POST ban `{"type":"temporary"}`（缺 duration_hours） | 400 |
| 3 | `AdminServer` | POST ban `{"type":"temporary","duration_hours":0}` | 400 |
| 4 | `AdminServer` | POST ban `{"type":"permanent","reason":"severe"}` 成功 | 200，DB 中 ban_until IS NULL 或 ban_type=`permanent` |
| 5 | `AdminServer` | 再次 POST ban | 200（幂等更新），admin_logs 新增 1 条 |
| 6 | `AdminServer` | CS_TOKEN 发起 ban | 403 code=40301 |

**【数据清理】**
- Unban U1。

---

## TC-USER-00005：Admin 解封用户 - 状态恢复 + 审计
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1.status=`banned`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | PUT `/api/v1/admin/users/{U1}/unban` Body `{"reason":"处罚到期"}` OP_TOKEN | 200 code=0 |
| 2 | `DB` | `SELECT status, ban_until FROM users WHERE id={U1}` | status=`active`，ban_until=NULL |
| 3 | `DB` | admin_logs 最新一条 action=`user_unban` | 1 行，detail 含 reason |
| 4 | `AdminServer` | PUT unban 对一个 active 用户 | 200 幂等或 409 `ALREADY_ACTIVE`（按实现） |
| 5 | `AdminServer` | PUT unban Body `{}`（缺 reason） | 400 code=40003 |

**【数据清理】**
- 无。
