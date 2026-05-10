# 测试套件：协议合规验证（Protocol Compliance）

> **🛡️ 治理类用例（非黑盒业务 E2E）**：本文件属于 [_README.md §0.4](../_README.md#04-治理类audit--proto--wiring说明) 定义的「协议字段冻结集成校验」，由 Node.js 直发 WS 帧/Redis 帧验证 `deny_unknown_fields` / `snake_case` / `Ping/Pong ms` 等协议铁律，**不通过用户 UI 操作**。维护方为协议治理团队，e2e-runner 与 qa-coordinator 不调度本文件。

> **需求模糊点 (Ambiguity Notes)**：
> - PROTO-04 中"DEV 环境"的判断条件未在协议文档中明确定义（`NODE_ENV=development` 还是其他标志）；暂以 `NODE_ENV=development` 为准，若 Web 侧有专属标志需同步更新此用例。
> - PROTO-06 中 Redis `admin:events` channel 的消费方（AppServer 还是 AdminServer）文档未完全明确；两端均订阅时需在步骤中分别断言，本用例以 AppServer 消费端为主断言点。

---

## TC-PROTO-00001：Server 拒绝包含未知字段的 WS 消息（deny_unknown_fields）

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile，临时 PG + Redis）。
2. 测试 WS 客户端（如 `wscat` 或 `tests/cross-lang/.../helpers/ws-client.ts` 的 `AndroidWsClient`）使用有效 JWT 已建立 WS 连接。
3. 当前协议约定：所有 WS 信令 schema 均设置 `"additionalProperties": false`（见 `doc/protocol/schemas/ws/TakeMic.schema.json` 等）。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                   | 预期结果 (Assertion)                                                                                                                  |
| :------: | :---------- | :-------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------ |
|    1     | `AppServer` | 通过 WS 连接发送以下 JSON 帧（含未知字段 `extra_field`）：`{"type":"TakeMic","msg_id":"<uuid-v4>","payload":{"mic_index":0},"extra_field":"evil"}` | 连接未立即断开                                                                                                                        |
|    2     | `AppServer` | 等待 2 秒，接收 AppServer 返回的消息                                                                                                                | 收到错误响应帧，`type` 为 `TakeMicResult`（或通用错误帧），`code` ≠ 0；**或** 连接被服务端主动关闭，WS close code ∈ {1003, 4000-4099} |
|    3     | `AppServer` | 查阅 AppServer 服务端日志（`tracing` 输出）                                                                                                         | 日志中出现含 `unknown field` 或 `deserialization error` 的 ERROR/WARN 级日志，且包含 `extra_field` 或 `TakeMic`                        |
|    4     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE seat_index=0`                                                                                             | 返回空行（未知字段帧未导致非法上麦写入）                                                                                              |

**【数据清理】**
- 关闭测试 WS 连接。
- 无 DB 数据需清理（操作未成功写入）。

---

## TC-PROTO-00002：Server 拒绝 camelCase 字段名的 WS 消息

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 测试 WS 客户端已建立有效连接（携带有效 JWT）。
3. 协议约定：所有字段必须使用 `snake_case`（见 `doc/protocol/conventions.md §4`），schema 设置 `"additionalProperties": false`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                   | 预期结果 (Assertion)                                                                                                                       |
| :------: | :---------- | :-------------------------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 发送 camelCase 字段的 WS 帧：`{"type":"TakeMic","msg_id":"<uuid-v4>","payload":{"micIndex":0}}`（使用 `micIndex` 而非 `mic_index`）                | 连接保持（或关闭）                                                                                                                         |
|    2     | `AppServer` | 等待 2 秒，接收服务端回包                                                                                                                           | 回包 `code` ≠ 0（反序列化失败）；**或** 连接被关闭；**不得** 返回 `code=0` 成功响应                                                       |
|    3     | `AppServer` | 再发送另一个 camelCase 帧：`{"type":"JoinRoom","msg_id":"<uuid-v4>","payload":{"roomId":"<valid-uuid>"}}`（使用 `roomId` 而非 `room_id`）           | 回包 `code` ≠ 0；房间加入操作**不得**成功                                                                                                  |
|    4     | `AppServer` | 运行 CI audit 脚本：`npx ts-node scripts/audit/protocol-binding-audit.ts`                                                                           | 脚本 exit code = 0（即生产代码中无 camelCase 字段写法残留，audit 全绿）                                                                    |
|    5     | `DB`        | 执行 `SELECT count(*) FROM room_members WHERE joined_at > NOW() - INTERVAL '10 seconds'`                                                            | 返回 `0`（camelCase 帧未触发加入房间的 DB 写入）                                                                                           |

**【数据清理】**
- 关闭测试 WS 连接。
- 无 DB 写入需清理。

---

## TC-PROTO-00003：Android sealed class 收到未知信令时走 Unknown 兜底不崩溃

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Compatibility`
- **回归级别**：`P0`

**【前置条件】**
1. Android 模拟器或真机已启动，App 已安装并以正常账号登录。
2. App 已进入某房间（`RoomScreen` 可见，WS 连接已建立）。
3. 准备一个能向该 WS 连接注入自定义帧的测试辅助工具（或通过 Mock AppServer 注入帧）。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                        | 预期结果 (Assertion)                                                                                              |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 通过 Mock Server 或测试注入，向 Android 客户端推送一个未知信令帧：`{"type":"FutureFeatureXYZ","msg_id":"<uuid>","payload":{"data":"test"},"timestamp":1700000000000}` | 帧成功发出                                                                                                        |
|    2     | `Android`   | 观察 App 界面（5 秒内）                                                                                                                                  | App **不崩溃**（无 Force Close 弹窗、无 ANR），`RoomScreen` 保持可见，UI 无任何错误提示                            |
|    3     | `Android`   | 通过 `adb logcat` 过滤标签 `RoomViewModel` 或 `WsMessage`                                                                                                | 日志中出现类似 `Unknown signal type: FutureFeatureXYZ` 或 `Unhandled message type` 的 WARN/DEBUG 日志，无 FATAL   |
|    4     | `Android`   | 再发送一个正常已知帧：`{"type":"RoomMessage","timestamp":1700000000000,"payload":{"msg_id":"<uuid>","user_id":"<uuid>","content":"hello","nickname":"Test"}}` | Android 端 `RoomScreen` 聊天列表新增一条消息"hello"，UI 正常渲染                                                  |

**【数据清理】**
- 无 DB 数据需清理。
- 关闭 Mock Server（如有）。

---

## TC-PROTO-00004：Web Zod 校验失败时 DEV 环境抛错、PROD 环境打日志不崩溃

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. Web 前端已在 **DEV 模式**（`NODE_ENV=development`）运行，访问地址 `http://localhost:5173`（或对应开发端口）。
2. 浏览器 DevTools Console 已打开，Filter 为 "All levels"。
3. 准备能拦截/注入 WS 消息的测试手段（如 `cy.intercept` 或手动 Mock WS Server）。

**【执行步骤与断言】**
| 步骤序号 | 目标端  | 操作动作 (Action)                                                                                                               | 预期结果 (Assertion)                                                                                                                                |
| :------: | :------ | :------------------------------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Web`   | 在 DEV 模式下，通过 Mock WS 向 Web 注入格式非法的 `MicTaken` 帧（payload 缺少 required 字段 `mic_index`）：`{"type":"MicTaken","timestamp":1700000000000,"payload":{"user_id":"<uuid>"}}` | —                                                                                                                                                   |
|    2     | `Web`   | 观察浏览器 Console（5 秒内）                                                                                                    | Console 中出现 **Error** 级别报错，内容包含 `ZodError` 或 `Validation failed` 以及 `mic_index` 字段名；页面**不崩溃**（白屏不可接受）              |
|    3     | `Web`   | 将 Web 环境切换为 **PROD 模式**（`NODE_ENV=production`，重新构建并运行），注入同样的非法帧                                      | —                                                                                                                                                   |
|    4     | `Web`   | 观察浏览器 Console（5 秒内）                                                                                                    | Console 中出现 **console.warn** 或 **console.error** 级日志，包含 `validation` 或 `schema` 相关文本；页面**不崩溃**，用户无感知（无 UI 错误提示）  |

**【数据清理】**
- 无 DB 数据需清理。
- 关闭 Mock WS Server。

---

## TC-PROTO-00005：Ping/Pong 三端 timestamp 均为毫秒级（断言 > 1_000_000_000_000）

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. Android 模拟器/真机 App 已登录，WS 连接已建立（`OkHttpWebSocketClient` 心跳已启动）。
3. Web 浏览器已打开 App，WS 连接已建立（如 Web 有 WS 心跳实现；若无则该端标注 N/A）。
4. 测试工具（`wscat`/`AndroidWsClient`）可捕获 WS 帧。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                     | 预期结果 (Assertion)                                                                                                                                     |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 使用测试 WS 客户端发送 `Ping` 帧：`{"type":"Ping","msg_id":"<uuid-v4>","timestamp":1700000000000}`                                   | 服务端接收成功                                                                                                                                           |
|    2     | `AppServer` | 接收 AppServer 回包 `Pong`                                                                                                            | 回包 JSON 字段：`type` = `"Pong"`，`msg_id` 与请求 `msg_id` 相同，`timestamp` 类型为 integer 且 **> 1,000,000,000,000**（断言为毫秒级，非秒级）         |
|    3     | `Android`   | 等待 Android `OkHttpWebSocketClient` 的自动心跳触发（默认 30s，测试时可注入短间隔 5s），通过 `adb logcat` 过滤 `OkHttpWebSocketClient` | logcat 中出现 `Pong received, timestamp=<N>`，其中 `<N>` **> 1,000,000,000,000**                                                                        |
|    4     | `Android`   | 检查 Android 发出的 `Ping` 帧（logcat 中记录的发送帧 JSON）                                                                           | 发送帧中若包含 `timestamp` 字段，其值 **> 1,000,000,000,000**（`System.currentTimeMillis()` 产生）                                                       |
|    5     | `AppServer` | 连续发送 5 次 `Ping`（间隔 1 秒），记录每次 `Pong.timestamp`                                                                          | 5 次 `Pong.timestamp` 值均 > 1,000,000,000,000，且相邻两次差值 ∈ [900, 1100]ms（误差 ≤ 100ms），无异常跳变                                              |

**【数据清理】**
- 无 DB 数据需清理。
- 关闭测试 WS 连接。

---

## TC-PROTO-00006：Redis admin:events 发布/消费双端字段完全对齐（BanUser/UnbanUser/CloseRoom/BroadcastNotice）

**【元数据】**
- **归属模块**：`PROTO`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. Redis 已启动，AppServer 已订阅 `admin:events` channel。
2. AdminServer 已启动，管理员账号 `E2E_ADMIN_ID`（UUID）已在 DB 中存在，角色为 `super_admin`。
3. DB 中存在测试用户 `E2E_TARGET_USER_ID`（UUID）和测试房间 `E2E_ROOM_ID`（UUID，状态 `live`）。
4. Redis 客户端工具（`redis-cli`）可用于发布消息和监听频道。
5. Schema 文件路径：`doc/protocol/schemas/pubsub/BanUser.schema.json`、`UnbanUser.schema.json`、`CloseRoom.schema.json`、`BroadcastNotice.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                                                         | 预期结果 (Assertion)                                                                                                                                             |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AdminServer` | 调用封禁用户接口（或直接通过 `redis-cli PUBLISH admin:events '{"type":"ban_user","payload":{"user_id":"<E2E_TARGET_USER_ID>"},"admin_id":"<E2E_ADMIN_ID>","ts":1700000000000}'`）                                         | Redis 发布成功，返回订阅者数量 ≥ 1                                                                                                                               |
|    2     | `AppServer` | 监听 AppServer 日志或通过 Redis SUBSCRIBE 旁路捕获消息内容                                                                                                                                                                | 消费的消息 JSON 字段完全符合 `BanUser.schema.json`：`type="ban_user"`（snake_case），`payload.user_id` 为 UUID 格式，`admin_id` 为 UUID 格式，`ts` 为整数 > 1,000,000,000,000 |
|    3     | `DB`        | 执行 `SELECT banned FROM users WHERE id='<E2E_TARGET_USER_ID>'`                                                                                                                                                           | 返回 `true`（AppServer 消费事件后正确更新 DB）                                                                                                                   |
|    4     | `AdminServer` | 发布解封事件：`redis-cli PUBLISH admin:events '{"type":"unban_user","payload":{"user_id":"<E2E_TARGET_USER_ID>"},"admin_id":"<E2E_ADMIN_ID>","ts":1700000000000}'`                                                       | Redis 发布成功                                                                                                                                                   |
|    5     | `AppServer` | 捕获 `admin:events` 消费消息内容                                                                                                                                                                                          | 消息字段符合 `UnbanUser.schema.json`：`type="unban_user"`，`payload.user_id` 为 UUID                                                                             |
|    6     | `AdminServer` | 发布关闭房间事件：`redis-cli PUBLISH admin:events '{"type":"close_room","payload":{"room_id":"<E2E_ROOM_ID>"},"admin_id":"<E2E_ADMIN_ID>","ts":1700000000001}'`                                                          | Redis 发布成功                                                                                                                                                   |
|    7     | `AppServer` | 捕获 `admin:events` 消费消息内容，并等待 3 秒                                                                                                                                                                             | 消息字段符合 `CloseRoom.schema.json`：`type="close_room"`，`payload.room_id` 为 UUID；AppServer 日志出现 `Room closed` 相关日志                                  |
|    8     | `DB`        | 执行 `SELECT status FROM rooms WHERE id='<E2E_ROOM_ID>'`                                                                                                                                                                  | 返回 `closed`（房间状态已更新）                                                                                                                                  |
|    9     | `AdminServer` | 发布全局公告事件：`redis-cli PUBLISH admin:events '{"type":"broadcast_notice","payload":{"message":"系统维护通知"},"admin_id":"<E2E_ADMIN_ID>","ts":1700000000002}'`                                                     | Redis 发布成功                                                                                                                                                   |
|   10     | `AppServer` | 捕获 `admin:events` 消费消息内容                                                                                                                                                                                          | 消息字段符合 `BroadcastNotice.schema.json`：`type="broadcast_notice"`，`payload.message` 非空字符串，`admin_id` 为 UUID，`ts` 为整数 > 1,000,000,000,000         |

**【数据清理】**
- `psql -c "UPDATE users SET banned=false WHERE id='<E2E_TARGET_USER_ID>'"`
- `psql -c "UPDATE rooms SET status='live' WHERE id='<E2E_ROOM_ID>'"` （如需恢复）
