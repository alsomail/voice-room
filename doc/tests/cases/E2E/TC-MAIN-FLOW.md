# 测试套件：主流程串联（Main Flow：登录 → 大厅 → 进房 → 上麦 → 送礼 → 余额联动 → AdminWeb 审计）

> **设计目标**：本套件唯一目的是**跑通主流程**，并验证**独立模块之间真的能串联起来**。
>
> 反例（必须避免）：房间功能单测全过、登录单测全过、钱包单测全过，但「点击进入房间」环节失败、模块之间断链。
>
> **铁律**：每条断言都必须能映射到**真实存在的代码或资源**，证据来源标注在每个步骤右侧。
>
> **真实资源对照（已读源代码）**：
> - AppServer 路由：`POST /api/v1/auth/verification-codes`、`POST /api/v1/auth/login`、`GET /api/v1/rooms`、`GET /api/v1/wallet/balance`、`POST /api/v1/gifts/send` 均已在 [app/server/src/modules/](../../../app/server/src/modules/) 实现；
> - WS 帧：`JoinRoom` / `JoinRoomResult` / `TakeMic` / `MicTaken` / `SendGift` / `GiftReceived` / `BalanceUpdated` 见 [doc/protocol/websocket_signals.md](../../protocol/websocket_signals.md)；
> - AdminServer 路由：`POST /api/v1/admin/login`、`GET /api/v1/admin/users/{id}`、`GET /api/v1/admin/users/{id}/events`、`GET /api/v1/admin/stats/overview` 均已在 [app/adminServer/src/](../../../app/adminServer/src/) 实现；**`/stats/timeseries` 不存在**；
> - Android 顶层 NavHost startDestination=`splash`，主路由：`splash` / `login` / `main` / `room/{roomId}`，定义于 [AppNavGraph.kt](../../../app/android/app/src/main/java/com/voice/room/android/presentation/AppNavGraph.kt)；
> - Web 路由：`/login` / `/dashboard` / `/rooms` / `/users` / `/logs`，受 RBAC 控制的 `/gifts`（super_admin/operator）+ `/rooms/governance`（super_admin/operator/cs），定义于 [router/index.tsx](../../../app/web/src/router/index.tsx) 与 [AppLayout.tsx](../../../app/web/src/app/AppLayout.tsx)；
> - 真实 fixtures（RUNBOOK §11.1）：User A=`+966500000900`（10w 金币 + 10w 钻石）、User B=`+966500000901`（0/0）；Admin=`super_admin / admin_password_change_me`；
> - OTP 注入（RUNBOOK L587）：`docker exec vr-redis redis-cli HSET sms:code:<phone> code 123456 attempts 0` + `EXPIRE 300`，**冷却键** `sms:cooldown:<phone>` 需要先 DEL 才能重发。
>
> **需求模糊点 (Ambiguity Notes)**：
> - 本套件先假定 RTC SDK 占位（不要求真实音频流），上麦只验证 `MicTaken` 广播 + UI 麦位状态。
> - 不验证真实充值（Phase 1 占位），User A seed 自带 10 万钻石足以覆盖 1 次低价送礼。

---

## TC-MAIN-FLOW-00001：登录主链路（Android：splash → login → main）
**【元数据】**
- **归属模块**：`AUTH × NAV`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 运行于 `:3000`，Redis/PG 健康（`curl http://localhost:3000/health` 返回 200）。
2. Android App 已安装；测试前调用 `resetAndroidToLoginPage()` 清除 JWT 并 force-stop。
3. 注入 OTP：
   - `docker exec vr-redis redis-cli DEL sms:cooldown:+966500000900`
   - `docker exec vr-redis redis-cli HSET sms:code:+966500000900 code 123456 attempts 0`
   - `docker exec vr-redis redis-cli EXPIRE sms:code:+966500000900 300`

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 启动 App | Splash 自动跳转，最终停留在 LoginScreen（顶部 logo + 手机号输入框） |
| 2 | `Android` | 在手机号输入框输入 `500000900`（前缀 `+966` 已固定） | 「获取验证码」按钮变为可点击 |
| 3 | `Android` | 点击「获取验证码」按钮 | 按钮变灰 60s 倒计时；接口 `POST /api/v1/auth/verification-codes` 命中（虽 OTP 已预注入，仍要走完真实接口） |
| 4 | `Android` | 在验证码输入框输入 `123456`，点击「登录」 | 进入 MainScreen（默认 `MainTab.ROOMS` 大厅 Tab） |
| 5 | `AppServer` | access log | `POST /api/v1/auth/login 200`，响应体含 `data.token`（JWT 形态） |
| 6 | `Android` | 验证 JWT 持久化 | `adb shell run-as ${ANDROID_APP_ID} cat /data/data/${ANDROID_APP_ID}/files/datastore/auth.preferences_pb` 输出非空（含 token） |
| 7 | `Android` | 杀掉 App 进程后再次启动 | Splash 直接跳到 MainScreen（无需再登录），验证 JWT 持久化生效 |

**【数据清理】**
- 调用 `resetAndroidToLoginPage()`，再次 DEL Redis 注入键。

---

## TC-MAIN-FLOW-00002：大厅 → 进房 → 上麦 → 送礼 → 余额联动（C 端串联主链路）
**【元数据】**
- **归属模块**：`ROOM × MIC × GIFT × WALLET`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. TC-MAIN-FLOW-00001 通过（User A 已登录，停留在 MainScreen 大厅 Tab）。
2. seed 数据库中存在至少 1 个房间（参见 RUNBOOK §11；如不存在，先用 super_admin 在 AdminWeb `/rooms` 创建一个，房主任选）。
3. 该房间至少 1 个空麦位。
4. **第二台设备 / 第二会话**：User B（`+966500000901`）作为接收方需在房间内（可由 e2e-runner 复用同一台 Android 通过 ADB 连接 + 切换账号或仅起一个 WS 后台连接验证；若 env 不具备多设备，**此用例 P0 部分降级为 single-device + AppServer 直接断言广播**）。
5. 记录 User A 当前钻石余额：`curl -H "Authorization: Bearer $TOKEN_A" http://localhost:3000/api/v1/wallet/balance`，记为 `BAL_BEFORE`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 大厅 Tab 列表加载完成 | 至少出现 1 张 RoomCard（含 `room_card_id-{X}` testTag，依据 [HallScreenTest.kt](../../../app/android/app/src/androidTest/java/com/voice/room/android/feature/room/HallScreenTest.kt) 已存在的同名 testTag 约定） |
| 2 | `AppServer` | access log | `GET /api/v1/rooms?page=1` 返回 200，`items[]` 非空 |
| 3 | `Android` | 点击第一张 RoomCard | NavController 跳转到 `room/{roomId}`，房间页加载麦位区域 |
| 4 | `AppServer` | WS log | 客户端发送 `JoinRoom` 帧（`room_id`=点击的房间）；服务端返回 `JoinRoomResult` 帧含 `code:0`，且广播 `UserJoined` |
| 5 | `Android` | 房间页 UI | MicSlots 区域可见；至少有一个空麦位（`mic_slot_empty_{N}` testTag 出现，参考 [MicSlotCardTest.kt](../../../app/android/app/src/androidTest/java/com/voice/room/android/feature/room/MicSlotCardTest.kt)） |
| 6 | `Android` | 点击空麦位（如 `mic_slot_empty_2`） | 系统提示授予麦克风权限 → 授予后发送 `TakeMic` 帧 |
| 7 | `AppServer` | WS log | 收到 `TakeMic`(`room_id`,`slot_id`=2)；返回 `TakeMicResult` `code:0`；广播 `MicTaken`(`user_id`=A,`slot_id`=2) |
| 8 | `Android` | 房间页 UI | `mic_slot_empty_2` 消失，被 `mic_slot_occupied_*` 替代，显示 User A 头像/昵称 |
| 9 | `Android` | 打开礼物面板，选择最低价礼物（gift_id=1 / quantity=1），接收方选择麦位上任意他人（若仅自己上麦，则跳过点击改为接收方=房主自动） | 「送出」按钮可点击 |
| 10 | `Android` | 点击「送出」 | 房间内播放礼物动画；`SendGift` 帧通过 WS 发送（参考 [websocket_signals.md](../../protocol/websocket_signals.md)） |
| 11 | `AppServer` | WS log + DB | `SendGiftResult` 帧 `code:0`，`balance_after` 字段已扣减；`wallet_transactions` 表新增至少 1 条 `type=gift_send` 记录（user_id=A）；房间内广播 `GiftReceived` |
| 12 | `Android` | 房间页钱包/余额展示位 | 数字从 `BAL_BEFORE` 滚动到 `BAL_BEFORE - gift.price` |
| 13 | `AppServer` | WS log | 客户端 A 收到点对点 `BalanceUpdated` 帧，`coin_balance` 或 `diamond_balance` 字段值 = `BAL_BEFORE - gift.price` |
| 14 | `AppServer` | HTTP 二次断言 | `curl -H "Authorization: Bearer $TOKEN_A" http://localhost:3000/api/v1/wallet/balance` 返回值 = `BAL_BEFORE - gift.price`（与 WS 推送一致） |

**【数据清理】**
- 调用 `LeaveMic` + `LeaveRoom`（或直接 force-stop）；
- 不强求恢复余额（User A seed 余额充足，单次低价礼物影响可忽略）；如需恢复，由 AdminWeb 用 super_admin 走 `POST /api/v1/admin/users/{A}/wallet/adjust` 补回。

---

## TC-MAIN-FLOW-00003：AdminWeb 联动审计（B 端 → C 端动作可见）
**【元数据】**
- **归属模块**：`ADMIN × USER × STATS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. TC-MAIN-FLOW-00002 刚刚跑完，User A 在过去 5 分钟内有过 1 次送礼。
2. AdminServer 运行于 `:3001`；AdminWeb dev server 运行于 `:5173`。
3. Web E2E 浏览器已就绪（playwright + midscene）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 浏览器访问 `http://localhost:5173/login` | 渲染登录表单（用户名 + 密码） |
| 2 | `AdminWeb` | 输入用户名 `super_admin`，密码 `admin_password_change_me`，点击「登录」 | 跳转到 `/dashboard`，左侧 Sider 渲染 6 个菜单：仪表盘 / 房间管理 / 用户管理 / 操作日志 / 礼物管理 / 治理日志（依据 [AppLayout.tsx](../../../app/web/src/app/AppLayout.tsx) 真实菜单逻辑） |
| 3 | `AdminServer` | access log | `POST /api/v1/admin/login 200` 命中，响应含 `token` |
| 4 | `AdminWeb` | 点击左侧「用户管理」菜单 | 进入 `/users`，列表加载（`GET /api/v1/admin/users 200` 命中） |
| 5 | `AdminWeb` | 在搜索框输入 `+966500000900`，进入 User A 详情 | 详情面板打开，显示 `coin_balance` / `diamond_balance` 字段；钻石余额 = `BAL_BEFORE - gift.price`（与 C 端 WS 推送一致） |
| 6 | `AdminServer` | access log | `GET /api/v1/admin/users/{A}` 200 命中 |
| 7 | `AdminWeb` | 点击左侧「仪表盘」 | 卡片渲染 `online_users` / `active_rooms` / `total_users` / `total_revenue` 四项数值（依据 `GET /api/v1/admin/stats/overview` 真实响应字段；**不再断言** `/stats/timeseries`，该接口不存在） |
| 8 | `AdminWeb` | 点击左侧「操作日志」 | 进入 `/logs`，`GET /api/v1/admin/logs 200` 命中（仅 super_admin 可见） |

**【数据清理】**
- 调用 AdminWeb logout 清空 token；浏览器 close。

---

## TC-MAIN-FLOW-00004：跨模块串联反例守卫（确保「点击进房」真的能跳过去）
**【元数据】**
- **归属模块**：`NAV × ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

> 本用例专门针对用户原话："房间功能测试完整，登录功能测试完整，测试通过，但实际上点击进入房间失败，功能无法串联"。
>
> 通过组合 3 个最小步骤的负向断言，确保 **MainScreen → RoomCard → RoomScreen 链路真实可达**，**不允许仅靠单端 mock 通过**。

**【前置条件】**
1. User A 已登录，停在 MainScreen 大厅 Tab。
2. AppServer 运行；至少 1 个真实房间存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 大厅 Tab 第一张房间卡片可点击 | testTag 形如 `room_card_id-*` 真实存在于 UI 树（不是 mock 数据） |
| 2 | `Android` | 点击该卡片 | 1.5s 内 NavController 跳到 `room/{roomId}`（URL 含真实 roomId 数字）；MicSlots 区域可见 |
| 3 | `AppServer` | WS log | 在跳转后的 3s 内必须命中 `JoinRoom` 帧；如未命中 → **本用例必须直接 FAIL** 标记为「主链路断裂」 |
| 4 | `Android` | 按返回键 | 退到 MainScreen（不能是空白页 / 不能崩溃） |
| 5 | `AppServer` | WS log | 收到 `LeaveRoom` 帧（验证 NavController 退出钩子正确触发） |

**【数据清理】**
- `resetAndroidToMainPage()`。
