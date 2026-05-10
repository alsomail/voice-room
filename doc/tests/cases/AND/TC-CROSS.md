# 测试套件：Android × Server 跨语言 E2E（Cross-Lang WS Loopback）

> **🛡️ 治理类用例（非黑盒业务 E2E）**：本文件属于 [_README.md §0.4](../_README.md#04-治理类audit--proto--wiring说明) 定义的「协议跨语言契约审计」，由 Node.js 模拟 Android client 直接发 WS 帧验证字段对齐，**不通过 Android UI 操作**。维护方为 server / android 协议团队，**不计入业务回归矩阵**，e2e-runner 与 qa-coordinator 不调度本文件。

> **需求模糊点 (Ambiguity Notes)**：
> - T-00104 定义的场景 #7（MuteUser→UserMuted）在 TDS 中描述为"Admin 通过 WS 触发"，但当前测试套件（CROSS-7）使用独立 admin token 的 WS 连接触发；若后续 Admin 走 HTTP 接口触发，本套件需同步更新。
> - `GiftReceived.schema.json` 文件不存在于 `doc/protocol/schemas/ws/`（T-00104 §4.3 差异 D-04），CROSS-06 采用结构性断言（字段存在性）而非 AJV Schema 全量校验。
> - `UserLeft` 广播与 `UserKicked` 点对点推送字段不同，CROSS-08 需双端断言。

---

## TC-CROSS-00001：JoinRoom → UserJoined payload 字段级断言（user_id/avatar snake_case）

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile，临时 PG + Redis），`E2E_SERVER_URL` 已配置。
2. 用户 A（`E2E_VALID_TOKEN`）和用户 B（`E2E_VALID_TOKEN_B`）均为有效账号，已在 DB 中存在。
3. 用户 A 使用有效 token 建立 WS 连接并已成功加入房间 `E2E_ROOM_ID`（已在 DB 中存在，状态 `live`）。
4. 用户 B 的 WS 连接已建立但**尚未加入**房间。
5. 测试工具：`tests/cross-lang/android-server-ws/helpers/ws-client.ts` 中的 `AndroidWsClient`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                            | 预期结果 (Assertion)                                                                                                                                                          |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 用户 B 通过 WS 发送 `JoinRoom` 帧：`{"type":"JoinRoom","msg_id":"<uuid-v4>","payload":{"room_id":"<E2E_ROOM_ID>"}}`                                                                         | 帧发出成功                                                                                                                                                                    |
|    2     | `AppServer` | 用户 B 等待接收 `JoinRoomResult` 响应（超时 5s）                                                                                                                                             | 收到帧，`type="JoinRoomResult"`，`code=0`，`msg_id` 与步骤 1 请求的 `msg_id` 相同                                                                                             |
|    3     | `AppServer` | 用户 A 等待接收 `UserJoined` 广播（超时 5s）                                                                                                                                                 | 收到帧，字段断言（对照 `UserJoined.schema.json`）：`type="UserJoined"`；`payload.user_id` 为 UUID 格式（snake_case，非 `userId`）；`payload.nickname` 为字符串；`payload.avatar` 类型为 string 或 null；`timestamp` 整数 > 1,000,000,000,000 |
|    4     | `AppServer` | 对步骤 3 收到的 `UserJoined` 帧执行 AJV Schema 全量校验，引用 `doc/protocol/schemas/ws/UserJoined.schema.json`                                                                              | AJV 校验通过（0 errors）；additionalProperties 约束未被违反（无多余字段）                                                                                                     |
|    5     | `DB`        | 执行 `SELECT count(*) FROM room_members WHERE room_id='<E2E_ROOM_ID>' AND user_id='<USER_B_ID>'`                                                                                            | 返回 `1`                                                                                                                                                                      |

**【数据清理】**
- `psql -c "DELETE FROM room_members WHERE room_id='<E2E_ROOM_ID>' AND user_id IN ('<USER_A_ID>', '<USER_B_ID>')"`
- 关闭用户 A、B 的 WS 连接。

---

## TC-CROSS-00002：SendMessage → RoomMessage 广播字段级断言

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 A（发送方）和用户 B（接收方旁观者）均已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. Schema 引用：`doc/protocol/schemas/ws/SendMessage.schema.json`、`doc/protocol/schemas/ws/RoomMessage.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                   | 预期结果 (Assertion)                                                                                                                                                                               |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 用户 A 发送 `SendMessage` 帧：`{"type":"SendMessage","msg_id":"<uuid-v4>","payload":{"content":"Hello Cross-Lang Test"}}`                                          | 帧发出成功                                                                                                                                                                                         |
|    2     | `AppServer` | 用户 A 接收 `SendMessageResult`（超时 3s）                                                                                                                          | 收到帧，`type="SendMessageResult"`，`code=0`                                                                                                                                                       |
|    3     | `AppServer` | 用户 B 接收 `RoomMessage` 广播（超时 3s）                                                                                                                           | 收到帧，字段断言（对照 `RoomMessage.schema.json`）：`type="RoomMessage"`；`payload.msg_id` 为 UUID 格式；`payload.user_id` 为 UUID 格式（snake_case，非 `userId`）；`payload.content="Hello Cross-Lang Test"`；`payload.nickname` 为字符串；`timestamp` 整数 > 1,000,000,000,000 |
|    4     | `AppServer` | 对步骤 3 的 `RoomMessage` 帧执行 AJV Schema 校验，引用 `doc/protocol/schemas/ws/RoomMessage.schema.json`                                                           | AJV 校验通过（0 errors）                                                                                                                                                                           |
|    5     | `AppServer` | 发送含 Unicode/emoji 内容：`{"type":"SendMessage","msg_id":"<uuid-v4-2>","payload":{"content":"🎉 مرحباً بالعالم"}}` ，用户 B 等待接收广播                        | 用户 B 收到 `RoomMessage`，`payload.content="🎉 مرحباً بالعالم"`（Unicode 字符无损传输）                                                                                                           |

**【数据清理】**
- 关闭 WS 连接；无 DB 写入需手动清理（消息记录可保留或按需删除）。

---

## TC-CROSS-00003：TakeMic → MicTaken payload 字段级断言（mic_index/user_id/forced_by）

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 A 和用户 B 均已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 麦位 2（`seat_index=2`）当前为空（DB 校验：`mic_seats` 表中 `seat_index=2` 的 `user_id` 为 NULL）。
4. Schema 引用：`doc/protocol/schemas/ws/TakeMic.schema.json`、`doc/protocol/schemas/ws/MicTaken.schema.json`、`doc/protocol/schemas/ws/TakeMicResult.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                             | 预期结果 (Assertion)                                                                                                                                                                                              |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 用户 A 发送 `TakeMic` 帧：`{"type":"TakeMic","msg_id":"<uuid-v4>","payload":{"mic_index":2}}`（字段名 `mic_index`，非 `slot` 或 `micIndex`）                | 帧发出成功                                                                                                                                                                                                        |
|    2     | `AppServer` | 用户 A 接收 `TakeMicResult`（超时 3s）                                                                                                                        | 收到帧，`type="TakeMicResult"`，`code=0`，`payload.mic_index=2`                                                                                                                                                   |
|    3     | `AppServer` | 用户 B 接收 `MicTaken` 广播（超时 3s）                                                                                                                        | 收到帧，字段断言（对照 `MicTaken.schema.json`）：`type="MicTaken"`；`payload.mic_index=2`（类型 integer）；`payload.user_id` = 用户 A 的 UUID（snake_case，非 `userId`）；`payload.forced_by` 为 `null` 或字段缺省；`timestamp` 整数 > 1,000,000,000,000 |
|    4     | `AppServer` | 对步骤 3 的 `MicTaken` 帧执行 AJV Schema 校验，引用 `doc/protocol/schemas/ws/MicTaken.schema.json`                                                           | AJV 校验通过（0 errors）；`additionalProperties: false` 约束成立（无 `micIndex`、`slotIndex` 等 camelCase 残留字段）                                                                                               |
|    5     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=2`                                                                           | 返回用户 A 的 UUID                                                                                                                                                                                                |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=2"`
- 关闭 WS 连接。

---

## TC-CROSS-00004：LeaveMic → MicLeft payload 字段级断言

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 A 和用户 B 均已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 用户 A 当前占据麦位 1（`seat_index=1`，DB 确认）。
4. Schema 引用：`doc/protocol/schemas/ws/LeaveMic.schema.json`、`doc/protocol/schemas/ws/MicLeft.schema.json`、`doc/protocol/schemas/ws/LeaveMicResult.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                              | 预期结果 (Assertion)                                                                                                                                                                                                              |
| :------: | :---------- | :----------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 用户 A 发送 `LeaveMic` 帧：`{"type":"LeaveMic","msg_id":"<uuid-v4>","payload":{"mic_index":1}}`                               | 帧发出成功                                                                                                                                                                                                                        |
|    2     | `AppServer` | 用户 A 接收 `LeaveMicResult`（超时 3s）                                                                                        | 收到帧，`type="LeaveMicResult"`，`code=0`                                                                                                                                                                                         |
|    3     | `AppServer` | 用户 B 接收 `MicLeft` 广播（超时 3s）                                                                                          | 收到帧，字段断言（对照 `MicLeft.schema.json`）：`type="MicLeft"`；`payload.mic_index=1`（类型 integer）；`payload.user_id` = 用户 A 的 UUID（snake_case）；`payload.forced=false`（非强制下麦）；`payload.forced_by` 为 `null` 或字段缺省；`timestamp` 整数 > 1,000,000,000,000 |
|    4     | `AppServer` | 对步骤 3 的 `MicLeft` 帧执行 AJV Schema 校验，引用 `doc/protocol/schemas/ws/MicLeft.schema.json`                               | AJV 校验通过（0 errors）                                                                                                                                                                                                          |
|    5     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=1`                                            | 返回空（user_id IS NULL）                                                                                                                                                                                                         |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=1"` （若用例失败未自动清理）
- 关闭 WS 连接。

---

## TC-CROSS-00005：Ping → Pong timestamp 毫秒级往返

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 测试 WS 客户端（`AndroidWsClient`）使用有效 JWT 已建立连接。
3. Schema 引用：`doc/protocol/schemas/ws/Ping.schema.json`、`doc/protocol/schemas/ws/Pong.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                        | 预期结果 (Assertion)                                                                                                                                 |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 发送 `Ping` 帧：`{"type":"Ping","msg_id":"<uuid-v4-1>","timestamp":1700000000000}`                                                                      | 帧发出成功                                                                                                                                           |
|    2     | `AppServer` | 接收 `Pong` 响应（超时 3s）                                                                                                                              | 收到帧，`type="Pong"`；`msg_id` = 步骤 1 的 `msg_id`（回显）；`timestamp` 整数 **> 1,000,000,000,000**（毫秒级，非秒级 `1_700_000_000`）            |
|    3     | `AppServer` | 对步骤 2 的 `Pong` 帧执行 AJV Schema 校验，引用 `doc/protocol/schemas/ws/Pong.schema.json`                                                              | AJV 校验通过（0 errors）；无多余字段（`additionalProperties: false`）                                                                                |
|    4     | `AppServer` | 连续发送 5 次 `Ping`（间隔 500ms），每次记录 `Pong.timestamp`                                                                                            | 5 次 `Pong.timestamp` 均 > 1,000,000,000,000；相邻两次差值 ∈ [400, 700]ms；无超时失败                                                               |
|    5     | `AppServer` | 记录第 1 次 `Ping` 发送时的本地系统时间 `t_send`（毫秒），与 `Pong.timestamp` 相减                                                                       | `|Pong.timestamp - t_send|` < 5000ms（往返延迟 + 时钟漂移容忍 5s，本地测试环境应远小于此值）                                                         |

**【数据清理】**
- 关闭测试 WS 连接；无 DB 数据需清理。

---

## TC-CROSS-00006：GiftSend → GiftReceived 广播（AJV 全量 Schema 校验）

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

> ⬆️ **升级说明（2026-05-06）**：`GiftReceived.schema.json` 已补充（commit `1da6c3c`），本用例从「结构性存在性断言」升级为「AJV 全量 Schema 校验」，回归级别同步从 P1 升级为 **P0**。

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 A（赠送方，`E2E_VALID_TOKEN`）和用户 B（接收方，`E2E_VALID_TOKEN_B`）均已加入房间 `E2E_ROOM_ID`。
3. 用户 A 账号余额 ≥ 礼物单价（DB 确认：`wallets` 表 `balance` 字段）。
4. 礼物 `E2E_GIFT_ID`（UUID）在 DB 的 `gifts` 表中存在，单价已知。
5. Schema 引用：
   - `doc/protocol/schemas/ws/SendGift.schema.json`（C→S）
   - `doc/protocol/schemas/ws/SendGiftResult.schema.json`（S→C 回执）
   - `doc/protocol/schemas/ws/GiftReceived.schema.json`（S→Room 广播，**完整 AJV 校验**）

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :---------- | :---------------- | :------------------- |
| 1 | `AppServer` | 用户 A 通过 WS 发送 `SendGift` 帧：`{"type":"SendGift","msg_id":"<uuid-v4>","payload":{"room_id":"<E2E_ROOM_ID>","gift_id":"<E2E_GIFT_ID>","receiver_id":"<USER_B_ID>","count":1},"timestamp":<ms>}` | 帧使用 `receiver_id`（非 `to_user_id`）+ `count`（非 `amount`），符合 `SendGift.schema.json`；帧发出无报错 |
| 2 | `AppServer` | 用户 A 等待接收 `SendGiftResult`（超时 5s） | 收到帧，`type="SendGiftResult"`，`payload.code=0`（成功） |
| 3 | `AppServer` | 用户 B 等待接收 `GiftReceived` 广播（超时 5s） | 收到帧，**使用 AJV 对照 `GiftReceived.schema.json` 做全量 Schema 校验**，断言零 error；具体字段验证：`payload.gift_record_id` 为 UUID；`payload.sender.user_id` = 用户 A 的 UUID；`payload.receiver.user_id` = 用户 B 的 UUID；`payload.gift.id` = `E2E_GIFT_ID`；`payload.count` = 1；`payload.total_price` ≥ 1；`timestamp` > 1,000,000,000,000 |
| 4 | `AppServer` | 房间内用户 C（旁观者，如有）等待接收 `GiftReceived` 广播（超时 5s） | 旁观者同样收到该广播，payload 内容与用户 B 收到的一致（同一 `msg_id`） |
| 5 | `DB` | 执行 `SELECT balance FROM wallets WHERE user_id='<USER_A_ID>'` | 余额 = 初始余额 − (礼物单价 × 1)，DB 已扣减 |
| 6 | `DB` | 执行 `SELECT id FROM gift_records WHERE sender_id='<USER_A_ID>' ORDER BY created_at DESC LIMIT 1` | 存在新增记录，`receiver_id`=用户B UUID，`gift_id`=`E2E_GIFT_ID`，`count`=1 |

**【数据清理】**
- `psql -c "UPDATE wallets SET balance=balance+<gift_price> WHERE user_id='<USER_A_ID>'"` （恢复余额）
- `psql -c "DELETE FROM gift_records WHERE sender_id='<USER_A_ID>' AND gift_id='<E2E_GIFT_ID>'"` （清理礼物记录）
- 关闭所有 WS 连接。

---

## TC-CROSS-00007：AdminKick → UserKicked 广播字段断言

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 A（普通用户，`E2E_VALID_TOKEN`）已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 管理员/房主用户（`E2E_ADMIN_TOKEN`）已加入同一房间，WS 连接有效，角色为 `admin` 或 `owner`。
4. Schema 引用：`doc/protocol/schemas/ws/KickUser.schema.json`、`doc/protocol/schemas/ws/KickUserResult.schema.json`、`doc/protocol/schemas/ws/UserKicked.schema.json`、`doc/protocol/schemas/ws/UserLeft.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                          | 预期结果 (Assertion)                                                                                                                                                                                        |
| :------: | :---------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 管理员发送 `KickUser` 帧：`{"type":"KickUser","msg_id":"<uuid-v4>","payload":{"target_user_id":"<USER_A_ID>","duration_sec":60}}`                                                         | 帧发出成功（字段 `target_user_id`，非 `userId` / `user_id`）                                                                                                                                                |
|    2     | `AppServer` | 管理员接收 `KickUserResult`（超时 3s）                                                                                                                                                     | 收到帧，`type="KickUserResult"`，`code=0`                                                                                                                                                                   |
|    3     | `AppServer` | 用户 A（被踢方）接收 `UserKicked` 点对点推送（超时 3s）                                                                                                                                    | 收到帧，字段断言（对照 `UserKicked.schema.json`）：`type="UserKicked"`；`payload.room_id="<E2E_ROOM_ID>"`（UUID 格式）；`payload.reason` 为字符串；`payload.cooldown_sec=60`（integer）；`payload.operator_nickname` 为字符串；`timestamp` 整数 > 1,000,000,000,000 |
|    4     | `AppServer` | 对步骤 3 的 `UserKicked` 帧执行 AJV Schema 校验，引用 `doc/protocol/schemas/ws/UserKicked.schema.json`                                                                                    | AJV 校验通过（0 errors）                                                                                                                                                                                    |
|    5     | `AppServer` | 用户 A 的 WS 连接状态检测（检测 `close` 事件或 `disconnect` 事件，超时 5s）                                                                                                                | 用户 A 的 WS 连接被服务端关闭（`connection_close` 或 WS close 事件触发）                                                                                                                                    |
|    6     | `DB`        | 执行 `SELECT count(*) FROM room_members WHERE room_id='<E2E_ROOM_ID>' AND user_id='<USER_A_ID>'`                                                                                           | 返回 `0`（用户已从房间成员中移除）                                                                                                                                                                          |

**【数据清理】**
- `psql -c "DELETE FROM room_members WHERE room_id='<E2E_ROOM_ID>' AND user_id='<USER_A_ID>'"` （若未自动清理）
- `psql -c "DELETE FROM room_kick_cooldowns WHERE user_id='<USER_A_ID>'"` （清理冷却期记录）
- 关闭所有 WS 连接。

---

## TC-CROSS-00008：RoomClosed → 所有用户收到广播

**【元数据】**
- **归属模块**：`CROSS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer + AdminServer + Redis 均已启动（test profile）。
2. 用户 A 和用户 B 均已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 管理员账号 `E2E_ADMIN_ID` 有权执行关闭房间操作（角色 `super_admin` 或 `admin`）。
4. Schema 引用：`doc/protocol/schemas/pubsub/CloseRoom.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端        | 操作动作 (Action)                                                                                                                                                                                     | 预期结果 (Assertion)                                                                                                                             |
| :------: | :------------ | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AdminServer` | 通过 Redis 发布关闭房间事件（或调用 AdminServer API）：`redis-cli PUBLISH admin:events '{"type":"close_room","payload":{"room_id":"<E2E_ROOM_ID>"},"admin_id":"<E2E_ADMIN_ID>","ts":1700000000000}'` | Redis 发布成功，返回订阅者数量 ≥ 1                                                                                                               |
|    2     | `AppServer`   | 用户 A 的 WS 连接接收推送（超时 5s）                                                                                                                                                                  | 用户 A 收到 `RoomClosed` 广播帧（或等效的房间关闭通知信令）；`payload.room_id="<E2E_ROOM_ID>"`；`timestamp` 整数 > 1,000,000,000,000             |
|    3     | `AppServer`   | 用户 B 的 WS 连接接收推送（超时 5s）                                                                                                                                                                  | 用户 B **同样**收到相同的 `RoomClosed` 广播帧，字段内容与步骤 2 一致                                                                             |
|    4     | `AppServer`   | 检测用户 A 和用户 B 的 WS 连接状态（超时 3s）                                                                                                                                                         | 两个连接均被服务端关闭（`close` 事件触发，或 `disconnect` 事件）                                                                                  |
|    5     | `DB`          | 执行 `SELECT status FROM rooms WHERE id='<E2E_ROOM_ID>'`                                                                                                                                              | 返回 `closed`                                                                                                                                    |
|    6     | `DB`          | 执行 `SELECT count(*) FROM room_members WHERE room_id='<E2E_ROOM_ID>'`                                                                                                                                | 返回 `0`（所有成员已从房间清出）                                                                                                                  |

**【数据清理】**
- `psql -c "UPDATE rooms SET status='live' WHERE id='<E2E_ROOM_ID>'"` （恢复房间状态以供其他用例复用，或直接重建）
- `psql -c "DELETE FROM room_members WHERE room_id='<E2E_ROOM_ID>'"` （若成员记录未自动清理）
- 关闭所有 WS 连接。
