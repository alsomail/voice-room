# 测试套件：WALLET 钱包（API）

> **需求模糊点 (Ambiguity Notes)**：
> - Phase 1 无真实充值通道，`/wallet/recharge` 返回占位提示；调用后不落单、不变动余额。

覆盖 Task：T-00016（钱包查询）、T-00016B（WS BalanceUpdated）、T-10012（Admin 调整余额）。

---

## TC-WALLET-00001：GET /wallet/balance 返回余额
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1.coin_balance=1234，`TOKEN_U1` 有效。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/wallet/balance` Bearer TOKEN_U1 | 200，`data.coin_balance=1234` |
| 2 | `AppServer` | 不带 Authorization | 401 code=40101 |

**【数据清理】**
- 无。

---

## TC-WALLET-00002：GET /wallet/transactions 分页
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 有 25 条 wallet_transactions（按 created_at DESC）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/wallet/transactions?page=1&page_size=20` | 200，items 长度=20，total=25，首项 created_at 最大 |
| 2 | `AppServer` | GET `?page=2&page_size=20` | items 长度=5 |
| 3 | `AppServer` | GET `?page_size=101` | 400 code=40003 |
| 4 | `AppServer` | items[0] 字段 | 包含 id/type(income\|expense)/amount/reason/balance_after/ref_type/ref_id/created_at |

**【数据清理】**
- 无。

---

## TC-WALLET-00003：WS BalanceUpdated 多端推送
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 从两个设备同时登录（WS_A、WS_B 两条连接），余额=1000。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 触发一笔服务端扣款（发送礼物等）扣 100 | 在 500ms 内 WS_A 与 WS_B 均收到 `{"type":"BalanceUpdated","payload":{"coin_balance":900,...}}` |
| 2 | `AppServer` | WS_A 与 WS_B 各自的 msg_id | 独立生成，不相同 |
| 3 | `AppServer` | 相同 msg_id 重复投递 | 客户端可依赖 msg_id 去重；服务端不重复触发业务 |

**【数据清理】**
- 恢复余额。

---

## TC-WALLET-00004：Admin 调整余额 - 正/负值 + 事务原子性
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1.coin_balance=500。
2. OP_TOKEN 具备 WalletWrite 权限。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST `/api/v1/admin/users/{U1}/wallet/adjust` Body `{"delta":200,"reason":"活动奖励"}` OP_TOKEN | 200，data.new_balance=700 |
| 2 | `DB` | users.coin_balance | =700 |
| 3 | `DB` | wallet_transactions 最新一条 | amount=200，type=`income`，reason=`活动奖励`，ref_type=`admin_adjust` |
| 4 | `DB` | admin_logs 最新 | action=`wallet_adjust`，detail.delta=200 |
| 5 | `Redis` | MONITOR | PUBLISH `admin:events` type=`balance_updated` user_id=U1 |
| 6 | `AdminServer` | POST delta=-300 reason="违规扣罚" | 200，余额=400 |
| 7 | `AdminServer` | POST delta=-1000（余额不足） | 400 code=40290（INSUFFICIENT_BALANCE），余额仍=400，wallet_transactions 未新增 |
| 8 | `AdminServer` | POST delta=0 | 400 code=40003 |
| 9 | `AdminServer` | POST Body 无 reason | 400 |
| 10 | `AdminServer` | CS_TOKEN 调用 | 403 |

**【数据清理】**
- 恢复 U1 余额为原值。

---

## TC-WALLET-00005：Admin 调整余额 - 事务失败回滚
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 模拟 `wallet_transactions.INSERT` 在事务中失败（如通过故障注入或预占约束）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST wallet/adjust delta=100 | 500 或明确错误 |
| 2 | `DB` | users.coin_balance | 未变更（整体事务回滚） |
| 3 | `DB` | wallet_transactions | 无新增行 |

**【数据清理】**
- 关闭故障注入。
