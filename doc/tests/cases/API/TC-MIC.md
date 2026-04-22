# 测试套件：MIC 麦位管理（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 无（契约见 `doc/protocol/websocket_signals.md`）

覆盖 Task：T-00014（麦位管理：上麦/下麦/禁麦/解麦）。

---

## TC-MIC-00001：TakeMic - 空位上麦成功 + 广播
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 房间 R1 active，U1 已加入，麦位 3 号为空，未被锁。
2. U1 与 U2 都在 R1 内，WS 活跃。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 通过 WS 发送 `{"type":"TakeMic","payload":{"slot_index":3},"msg_id":"m1"}` | U1 收到 `{"type":"Ack","msg_id":"m1","code":0}` |
| 2 | `AppServer` | R1 内所有成员 | 收到 `{"type":"MicTaken","payload":{"user_id":"{U1}","slot_index":3}}` 广播 |
| 3 | `DB/Memory` | 查询麦位 3 | 占用者=U1 |

**【数据清理】**
- U1 下麦。

---

## TC-MIC-00002：TakeMic - 麦位被占返回错误
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 麦位 3 已被 U2 占用。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发送 TakeMic slot_index=3 | 收到 `{"type":"Ack","code":40907}` 或 `MIC_OCCUPIED` |
| 2 | `AppServer` | 其他成员 | 未收到 MicTaken 广播 |

**【数据清理】**
- 无。

---

## TC-MIC-00003：TakeMic - 被禁麦的用户无法上麦
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 房主 U1 对 U2 执行 MuteUser，状态记录 mic_muted=true。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发送 TakeMic slot_index=4 | 收到 `{"type":"Ack","code":40306}` 或 `MIC_MUTED` |
| 2 | `AppServer` | 房间广播 | 无 MicTaken |

**【数据清理】**
- 房主解除 mute。

---

## TC-MIC-00004：TakeMic - 并发请求同一空位仅一人成功
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 麦位 5 为空。
2. U1、U2、U3 均在房间内。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1/U2/U3 几乎同时发 TakeMic slot_index=5 | 仅 1 人收到 `code=0` 成功 Ack，另外 2 人收到 `code=40907` |
| 2 | `AppServer` | 房间广播 | 仅 1 条 `MicTaken slot_index=5` 广播 |
| 3 | `DB/Memory` | 查询麦位 5 占用者 | 单一用户，非空 |

**【数据清理】**
- 下麦清理。

---

## TC-MIC-00005：LeaveMic - 仅本人/房主可下麦
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. U2 占据麦位 3，U3 为普通成员。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U3 发 `{"type":"LeaveMic","payload":{"target_user_id":"{U2}"}}` | 收到 `code=40301`（无权限） |
| 2 | `AppServer` | 房主 U1 发同样 LeaveMic | 成功，广播 `MicLeft` |
| 3 | `AppServer` | U2 发 LeaveMic（自下） | 麦位重新空闲但幂等返回成功，或 `NOT_ON_MIC`（视实现） |

**【数据清理】**
- 无。

---

## TC-MIC-00006：MuteUser / TransferAdmin - 房主权限 + 幂等
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 房主 U1，成员 U2。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发 MuteUser target=U2 msg_id=x | 广播 `UserMuted`，U2 进入 mic_muted |
| 2 | `AppServer` | U1 重发同 msg_id=x | 服务端基于 msg_id 去重，不再广播，返回上次结果 |
| 3 | `AppServer` | U2 发 MuteUser target=U1 | `code=40301`（非房主） |
| 4 | `AppServer` | U1 发 TransferAdmin new_owner=U2 | 广播 `AdminTransferred`，R1.owner_id 内存态与 DB 同步更新为 U2 |
| 5 | `AppServer` | U1 再次发 MuteUser | 403 |

**【数据清理】**
- 将房主转回或关闭房间。
