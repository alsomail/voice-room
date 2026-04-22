# 测试套件：LOG 操作审计日志（Admin API）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-10010（admin_logs 查询）、T-10011（关键操作日志写入）。

---

## TC-LOG-00001：关键操作自动写入 admin_logs
**【元数据】**
- **归属模块**：`LOG`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. admin_op 执行多种敏感操作：登录、封禁、解封、强制关闭房间、调整余额。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | admin_op 登录 | admin_logs 有 action=`login` 记录，ip 字段非空 |
| 2 | `AdminServer` | admin_op ban 用户 U1 | 新增 action=`user_ban`，target_id=U1，detail JSON 含 reason/duration |
| 3 | `AdminServer` | admin_op unban U1 | 新增 action=`user_unban` |
| 4 | `AdminServer` | admin_op force-close R1 | 新增 action=`room_force_close`，target_id=R1 |
| 5 | `AdminServer` | admin_op 调整 U1 余额 | 新增 action=`wallet_adjust`，detail 含 delta 与 reason |

**【数据清理】**
- 无（审计日志保留）。

---

## TC-LOG-00002：查询日志 - 按操作类型 + 操作人 + 时间范围
**【元数据】**
- **归属模块**：`LOG`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. admin_logs 已积累 ≥50 条记录，时间跨 3 天。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/logs?page=1&page_size=20` OP_TOKEN | 200，按 created_at DESC 返回 20 条 |
| 2 | `AdminServer` | GET `?action=user_ban` | 所有 items.action=`user_ban` |
| 3 | `AdminServer` | GET `?admin_id={admin_op_id}&start_at={T-1d}&end_at={T}` | items.admin_id 均等于传入值，created_at 均在范围内 |
| 4 | `AdminServer` | GET `?page_size=101` | 400 code=40003 |
| 5 | `AdminServer` | GET CS_TOKEN | 200（CS 可读审计） |
| 6 | `AdminServer` | GET Bearer FIN_TOKEN 无 LogRead 权限 | 403 |

**【数据清理】**
- 无。

---

## TC-LOG-00003：日志查询性能 - 10 万行 ≤500ms
**【元数据】**
- **归属模块**：`LOG`
- **测试类型**：`Performance`
- **回归级别**：`P1`

**【前置条件】**
1. 向 admin_logs 插入 10 万行种子数据，created_at 索引就位。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/logs?page=1&page_size=20` 连续 10 次 | 每次响应时间 p95 ≤500ms |
| 2 | `DB` | `EXPLAIN` 查询 | 命中 created_at 索引（Index Scan） |

**【数据清理】**
- TRUNCATE admin_logs 测试数据。
