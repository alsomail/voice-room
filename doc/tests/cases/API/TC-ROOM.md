# 测试套件：ROOM 房间大厅与管理（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 无（契约见 `doc/protocol/room_api.md`、`doc/protocol/admin_api.md`）

覆盖 Task：T-00006~T-00010（App Server 房间 CRUD）、T-10004~T-10006（Admin 房间管理）。

---

## TC-ROOM-00001：创建房间 - 合法参数返回 201
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 用户 U1 登录获得 `TOKEN_U1`，DB 中无 U1 的活跃房间。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/rooms` Bearer TOKEN_U1 Body `{"title":"我的语聊房","room_type":"normal"}` | HTTP 201，`code=0`，`data.room_id` 为合法 UUID |
| 2 | `DB` | `SELECT owner_id, status, room_type, password_hash FROM rooms WHERE id=data.room_id` | 1 行，owner_id=U1，status=`active`，room_type=`normal`，password_hash IS NULL |

**【数据清理】**
- DELETE rooms WHERE id=data.room_id。

---

## TC-ROOM-00002：创建房间 - 标题长度边界值 (0/1/30/31)
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 登录，每步前清理其活跃房间。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST rooms title=`""` | HTTP 400，`code=40003` |
| 2 | `AppServer` | POST rooms title=`"a"`（Min=1） | HTTP 201 |
| 3 | `AppServer` | POST rooms title=30 个中文字符（Max=30） | HTTP 201 |
| 4 | `AppServer` | POST rooms title=31 个中文字符（Max+1） | HTTP 400，`code=40003` |

**【数据清理】**
- 每步后关闭/删除创建的房间。

---

## TC-ROOM-00003：创建房间 - room_type 枚举 + 密码字段处理
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. U1 有效 token，无活跃房间。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `{"title":"t","room_type":"hack"}` | HTTP 400，`code=40003` |
| 2 | `AppServer` | POST `{"title":"t","room_type":"password"}`（缺 password） | HTTP 400，`code=40003` |
| 3 | `AppServer` | POST `{"title":"t","room_type":"password","password":"123456"}` | HTTP 201 |
| 4 | `DB` | 查该房间 password_hash | 非空，以 `$2` 开头的 bcrypt 格式，绝非明文 |
| 5 | `AppServer` | POST `{"title":"t","room_type":"normal","password":"xyz"}`（normal 带密码） | HTTP 201 |
| 6 | `DB` | 查该房间 password_hash | IS NULL（忽略字段） |

**【数据清理】**
- 清理测试房间。

---

## TC-ROOM-00004：创建房间 - 同用户并发创建仅一个成功（幂等）
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 登录，无活跃房间。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 并发 5 个相同 POST `/api/v1/rooms` Bearer TOKEN_U1 | 恰好 1 个 HTTP 201，其余 4 个 HTTP 409 `code=40900` |
| 2 | `DB` | `SELECT count(*) FROM rooms WHERE owner_id=U1 AND status='active'` | =1 |

**【数据清理】**
- 关闭房间。

---

## TC-ROOM-00005：创建房间 - 未登录 / Token 过期
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 无；预备过期 token `EXP_TOKEN`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/rooms` 不带 Authorization | HTTP 401，`code=40101` |
| 2 | `AppServer` | POST `/api/v1/rooms` Bearer EXP_TOKEN | HTTP 401，`code=40102` |

**【数据清理】**
- 无。

---

## TC-ROOM-00006：房间列表 - 热度降序 + 分页
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 构造 25 个 active 房间，member_count 分别为 1-25。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/rooms?page=1&size=20` | `data.total=25`，`items` 长度=20，首项 member_count=25，末项=6 |
| 2 | `AppServer` | GET `?page=2&size=20` | `items` 长度=5，首项 member_count=5 |
| 3 | `AppServer` | GET `?page=999&size=20` | `total=25`，`items` 长度=0 |
| 4 | `AppServer` | GET `?page=0` | HTTP 400，`code=40003` |
| 5 | `AppServer` | GET `?size=0` | HTTP 400 |
| 6 | `AppServer` | GET `?size=101` | HTTP 400 |
| 7 | `AppServer` | GET `?size=100` | HTTP 200，items 长度=25 |

**【数据清理】**
- 清空测试房间。

---

## TC-ROOM-00007：房间列表 - 已关闭/软删除房间不可见
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. R1 active、R2 closed、R3 deleted_at 非空。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/rooms` | items 仅含 R1，不含 R2/R3 |

**【数据清理】**
- 无。

---

## TC-ROOM-00008：房间详情 - 合法/非法/不存在
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. R1 active，房主 U1。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/rooms/{R1}` | HTTP 200，`data.owner.user_id=U1`，`data.mic_slots=[]` |
| 2 | `AppServer` | GET `/api/v1/rooms/not-a-uuid` | HTTP 400，`code=40003` |
| 3 | `AppServer` | GET `/api/v1/rooms/00000000-0000-0000-0000-000000000000` | HTTP 404，`code=40400` |

**【数据清理】**
- 无。

---

## TC-ROOM-00009：关闭房间 - 权限 + 状态机
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. R1 active，owner=U1；U2 拥有 `TOKEN_U2`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | DELETE `/api/v1/rooms/{R1}` Bearer TOKEN_U2 | HTTP 403，`code=40301` |
| 2 | `DB` | 查 R1 status | 仍 `active` |
| 3 | `AppServer` | DELETE Bearer TOKEN_U1 | HTTP 200 |
| 4 | `DB` | 查 R1 status | `closed` |
| 5 | `AppServer` | 再次 DELETE Bearer TOKEN_U1 | HTTP 409，`code=40901` |
| 6 | `AppServer` | DELETE `/api/v1/rooms/{not_exist_uuid}` | HTTP 404，`code=40400` |

**【数据清理】**
- 无。

---

## TC-ROOM-00010：Admin 房间列表 - 筛选 + RBAC
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. DB 构造 5 active + 3 closed 房间；含 title 包含"测试"的房间 2 个。
2. OP_TOKEN、FIN_TOKEN、CS_TOKEN 可用；C 端 TOKEN_U1 可用。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/rooms?page=1&page_size=20` Bearer OP_TOKEN | HTTP 200，`data.total=8`，items 含 closed 房间 |
| 2 | `AdminServer` | GET `?status=closed` OP_TOKEN | total=3，所有 items.status=`closed` |
| 3 | `AdminServer` | GET `?status=invalid` OP_TOKEN | HTTP 400，`code=40003` |
| 4 | `AdminServer` | GET `?keyword=测试` OP_TOKEN | items 仅 2 条 |
| 5 | `AdminServer` | GET Bearer FIN_TOKEN | HTTP 403，`code=40301` |
| 6 | `AdminServer` | GET Bearer TOKEN_U1（C 端 JWT） | HTTP 401，`code=40101` |

**【数据清理】**
- 清理测试数据。

---

## TC-ROOM-00011：Admin 房间详情 - 可见 closed，404 软删除
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. R1 closed；R2 deleted_at 非空。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/rooms/{R1}` OP_TOKEN | HTTP 200，`data.status="closed"`，含 `updated_at` 字段 |
| 2 | `AdminServer` | GET `/api/v1/admin/rooms/{R2}` OP_TOKEN | HTTP 404，`code=40400` |

**【数据清理】**
- 清理数据。

---

## TC-ROOM-00012：Admin 强制关闭房间 - 全流程 + 审计
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 active；R2 closed；admin_op 使用 OP_TOKEN。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | DELETE `/api/v1/admin/rooms/{R1}` Bearer OP_TOKEN | HTTP 200，`code=0` |
| 2 | `DB` | `SELECT status FROM rooms WHERE id={R1}` | `closed` |
| 3 | `DB` | `SELECT * FROM admin_logs WHERE action='room_force_close' ORDER BY created_at DESC LIMIT 1` | 1 行，target_id=R1，admin_id=admin_op 对应 id |
| 4 | `Redis` | MONITOR 观察 | 可见 PUBLISH `admin:events` payload `{"type":"close_room","room_id":"{R1}"}` |
| 5 | `AdminServer` | DELETE `{R2}`（已 closed） OP_TOKEN | HTTP 409，`code=40901` |
| 6 | `AdminServer` | DELETE 不存在 id OP_TOKEN | HTTP 404，`code=40400` |
| 7 | `AdminServer` | DELETE `{R1_new_active}` Bearer FIN_TOKEN | HTTP 403，`code=40301` |

**【数据清理】**
- 清理数据。

---

## TC-ROOM-00013：WS JoinRoom - 加入内存态 + 广播 UserJoined + 返回快照
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 房间 R1 已由 U1 创建并 JoinRoom，麦上 U1 占 slot=1。
2. U2 WS 已连接但未进房。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 `{"type":"JoinRoom","payload":{"room_id":"R1"}}` | U2 收到 `RoomState` 快照，`mic_slots[0].user_id=U1`，`members_count≥2` |
| 2 | `AppServer` | U1 的 WS 通道 | 收到 `{"type":"UserJoined","payload":{"user_id":"U2",...}}` |
| 3 | `AppServer` | U2 再发 JoinRoom 同 R1（已在房间） | 返回 `code=40902, error=ALREADY_IN_ROOM` 或幂等返回同快照（以实现为准，断言二者之一） |
| 4 | `AppServer` | U2 发 JoinRoom room_id 不存在 | `code=40400, ROOM_NOT_FOUND` |
| 5 | `AppServer` | U2 发 JoinRoom 到 closed 房间 | `code=40910, ROOM_CLOSED` |

**【数据清理】**
- 无。

---

## TC-ROOM-00014：WS LeaveRoom - 显式离开 + 连接断开隐式离开 + 在麦自动下麦
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 U1，成员 U2（观众），麦上 U3 占 slot=2。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 `{"type":"LeaveRoom"}` | 房间广播 `UserLeft {user_id:U2}`；U2 自身 WS 连接保持 |
| 2 | `AppServer` | U3 WS 连接被网络强断（模拟 TCP RST），30s 内 | 服务端心跳超时后，房间广播 `MicLeft {slot:2}` → `UserLeft {user_id:U3}` |
| 3 | `AppServer` | U1（房主）发 LeaveRoom | 广播 `UserLeft {user_id:U1}`；房间不自动关闭（房主离开 ≠ 关闭房间） |
| 4 | `DB` | `SELECT status FROM rooms WHERE id=R1` | 仍为 `active` |
| 5 | `AppServer` | 非房间成员 U5 发 LeaveRoom | `code=40901, NOT_IN_ROOM` |

**【数据清理】**
- 关闭 R1。
