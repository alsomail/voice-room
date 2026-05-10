# 测试套件：E2E 装配契约（DI Wiring Contract）

> **🛡️ 治理类用例（与业务用例配合执行）**：本文件属于 [_README.md §0.4](../_README.md#04-治理类audit--proto--wiring说明) 定义的「装配契约审计」，验证 DI / NoOpRepository / onClick 等防腐层链路是否真实接通，**不验证业务行为本身**。维护方为架构治理团队，与对应模块业务用例配合执行。

> **需求模糊点 (Ambiguity Notes)**：
> - 本套件是新增的"装配体检层"，专门防御「Compose Screen 在 AppNavGraph 注册路由时漏接 ViewModel.Factory，静默回退到默认空实现 (NoOp / Preview Stub) 导致网络请求从未发出」类缺陷。
> - 本套件用例**必须**在真机/模拟器上以真实 `MainActivity` 启动，**禁止**使用 `composeTestRule.setContent { ... }` 隔离渲染单 Screen — 那种方式会绕过本套件要验证的装配链。
> - 本套件用例每条**必须**包含至少一条「服务端日志/DB 状态」副作用断言，仅 UI 文案断言不被认可（详见 `doc/tests/cases/_README.md` 铁律 6）。
> - 本套件归类为 `regression_level=P0`，**每次 PR 必跑**；任一例 FAIL 即阻断合入。

---

## TC-WIRING-00001：登录页持有真实 AuthRepository（防 NoOpAuthRepository 回退）

**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. App 全新安装（`adb shell pm clear ${ANDROID_APP_ID}`），无 JWT。
2. AppServer / Redis / Postgres preflight 全绿。
3. DB 无 phone=`+966500000900` 的用户；Redis 无 `sms:code:+966500000900` 与 `sms:cooldown:+966500000900`。
4. 启动一个独立的 AppServer access-log tail 旁路（`tail -F` AppServer 日志或开启 access-log endpoint 探针）用于 step 3/6 的副作用断言。

**【执行步骤与断言】**
| 步骤序号 | 目标端       | 操作动作 (Action)                                                                                       | 预期结果 (Assertion)                                                                                                                                            |
| :------: | :----------- | :------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`    | `adb shell am start -n ${ANDROID_APP_ID}/com.voice.room.android.presentation.MainActivity`              | App 进程启动；Splash 800ms 后页面显示 +966 国家码、`phone_input` testTag、`code_input` testTag、"获取验证码" 按钮                                                |
|    2     | `Android`    | 在手机号框输入 `500000900`，点击"获取验证码"                                                            | 按钮文案变为 "60s 后重发"（视觉计时器启动）                                                                                                                     |
|    3     | `AppServer`  | 在 step 2 后 5 秒内 tail AppServer access log                                                           | 出现一行包含 `POST /api/v1/auth/verification-codes` 且 status=`200` 且请求体含 `+966500000900`。**这一步是本用例的核心断言** — 缺它就退化为 NoOp 假倒计时       |
|    4     | `Redis`      | `redis-cli GET sms:code:+966500000900`                                                                  | 返回 6 位数字字符串，且 `TTL sms:code:+966500000900` ∈ (0, 300]                                                                                                 |
|    5     | `Redis`      | `redis-cli SET sms:code:+966500000900 123456 EX 300`（覆盖为已知值便于后续断言）                        | OK                                                                                                                                                              |
|    6     | `Android`    | 验证码框输入 `123456`，点击"登录"                                                                       | 按钮变 Loading                                                                                                                                                  |
|    7     | `AppServer`  | step 6 后 5 秒内 tail access log                                                                        | 出现 `POST /api/v1/auth/login` status=`200` body 含 `+966500000900` 与 `123456`                                                                                 |
|    8     | `DB`         | `psql -tA -c "SELECT count(*) FROM users WHERE phone='+966500000900'"`                                  | 返回 `1`                                                                                                                                                        |
|    9     | `Android`    | 2 秒内观察当前页面                                                                                      | 已离开 LoginScreen，显示 MainScreen（有 `Rooms / Messages / Me` 底部 Tab，标题文本 `VoiceRoom`，且至少一张 `room_card_*` testTag 可见）                          |
|    10    | `AppServer`  | tail access log                                                                                         | 出现 `GET /api/v1/rooms?page=1&size=20` status=`200` —— 证明登录后真实拉取房间列表的链路也走通                                                                  |

**【数据清理】**
- `psql -c "DELETE FROM users WHERE phone='+966500000900'"`
- `redis-cli DEL sms:code:+966500000900 sms:cooldown:+966500000900 sms:daily:+966500000900`
- `adb shell pm clear ${ANDROID_APP_ID}`

---

## TC-WIRING-00002：大厅房间卡片可点击进入房间（防漏接 onClick / 防错装 ViewModel）

**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 用户 U1（`E2E_USER_A`）已在 App 中登录（沿用 TC-WIRING-00001 收尾态或显式 seed）。
2. DB 中存在 `E2E_ROOM_ID` 房间，状态 `live`，房主非 U1。
3. AppServer access-log tail 已就绪。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                  | 预期结果 (Assertion)                                                                                                  |
| :------: | :---------- | :------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | App 已停留 MainScreen，底部 `Rooms` Tab 高亮                                                       | 顶部显示 `VoiceRoom`、`热门 / 新开 / 关注 / 游戏` Tab；网格中至少 1 张房间卡片，每张卡片可见房间标题与房主名           |
|    2     | `Android`   | 通过文本定位"E2E Test Room"卡片（不可使用 `tapOn: index: 0`，因 index:0 实际是 Tab "热门"）        | 视觉上点击命中房间卡片，过渡动画启动                                                                                  |
|    3     | `AppServer` | tail access log 5 秒                                                                               | 出现 `POST /api/v1/rooms/${E2E_ROOM_ID}/join` status=`200` 或对应 WS `room.join` 帧。**这一步必须有，否则用例失败**    |
|    4     | `Android`   | 3 秒内观察页面                                                                                     | 进入 RoomScreen：顶部显示房间标题 "E2E Test Room"，麦位区域 8 个 `mic_seat_*` testTag 可见，底部出现 `room_bottom_bar` |
|    5     | `DB`        | `psql -tA -c "SELECT count(*) FROM room_members WHERE room_id='${E2E_ROOM_ID}' AND user_id='${U1}'"` | 返回 `1`                                                                                                              |
|    6     | `Android`   | 点击返回                                                                                           | 回到 MainScreen Rooms Tab，房间卡片仍可见                                                                             |

**【数据清理】**
- `psql -c "DELETE FROM room_members WHERE room_id='${E2E_ROOM_ID}' AND user_id='${U1}'"`

---

## TC-WIRING-00003：FAB"创建房间"流程真实落库（防 onClick 空实现）

**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 已登录并停留在 MainScreen Rooms Tab。
2. DB `rooms` 表中不存在 title=`WIRING-CREATE-CHECK` 的房间。
3. AppServer access-log tail 已就绪。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                  | 预期结果 (Assertion)                                                                |
| :------: | :---------- | :--------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------- |
|    1     | `Android`   | 点击右下角金色 "+" FAB（`create_room_fab`）                                        | 弹出 BottomSheet "创建房间"                                                         |
|    2     | `Android`   | 标题输入 `WIRING-CREATE-CHECK`，分类点 `chat`，点击"创建"                          | 按钮 Loading                                                                        |
|    3     | `AppServer` | tail access log 5 秒                                                               | 出现 `POST /api/v1/rooms` status=`200` body 含 `WIRING-CREATE-CHECK`                |
|    4     | `DB`        | `psql -tA -c "SELECT count(*) FROM rooms WHERE title='WIRING-CREATE-CHECK'"`       | 返回 `1`                                                                            |
|    5     | `Android`   | 3 秒内观察页面                                                                     | 自动进入新房间 RoomScreen，标题 = `WIRING-CREATE-CHECK`，房主 = U1（房主麦位被占）  |

**【数据清理】**
- `psql -c "DELETE FROM rooms WHERE title='WIRING-CREATE-CHECK'"`

---

## TC-WIRING-00004：上麦操作真实触达 RTC + AppServer（防 RtcPort NoOp）

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 已进入 `${E2E_ROOM_ID}` 房间。
2. App 已授予 `RECORD_AUDIO` 权限（`adb shell pm grant ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`）。
3. 麦位 3 当前空位（DB 校验）。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                          | 预期结果 (Assertion)                                                                                  |
| :------: | :---------- | :--------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 点击 `mic_seat_3`                                                                                          | 弹出"上麦"确认或直接上麦动画                                                                          |
|    2     | `AppServer` | tail access/WS log 3 秒                                                                                    | 出现 WS 帧 `mic.up` 或 HTTP `POST /api/v1/rooms/${E2E_ROOM_ID}/mic/up` status=`200`                   |
|    3     | `DB`        | `psql -tA -c "SELECT user_id FROM mic_seats WHERE room_id='${E2E_ROOM_ID}' AND seat_index=3"`              | 返回 U1 的 user_id                                                                                    |
|    4     | `Android`   | 观察 `mic_seat_3` 视觉                                                                                     | 头像替换为 U1 头像，且出现 `rtc_publishing` 指示（声音波纹 / 推流图标）                                |
|    5     | `Android`   | 点击 `mic_seat_3_occupied` 自身位 → 选择"下麦"                                                             | 动画离麦                                                                                              |
|    6     | `AppServer` | tail log 3 秒                                                                                              | 出现 `mic.down` 帧或 `POST .../mic/down` status=`200`                                                 |
|    7     | `DB`        | 同 step 3 查询                                                                                             | 返回空（user_id IS NULL）                                                                             |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='${E2E_ROOM_ID}' AND seat_index=3"`

---

## TC-WIRING-00005：埋点上报真实命中 events/batch（防 AnalyticsPort NoOp）

**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. U1 已登录。
2. AppServer events/batch endpoint 已就绪。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                  | 预期结果 (Assertion)                                                                                              |
| :------: | :---------- | :--------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 触发任一可埋点动作（如 `login_verify_success` 已在 TC-WIRING-00001 后产生）        | App 内部 EventBatcher 入队                                                                                        |
|    2     | `AppServer` | 30 秒内 tail access log                                                            | 至少出现一次 `POST /api/v1/events/batch`，body 含 `event_name=login_verify_success`，status ∈ {200, 500}（容忍） |
|    3     | `DB`        | `psql -tA -c "SELECT count(*) FROM analytics_events WHERE event_name='login_verify_success' AND user_id='${U1}'"` | ≥ 1                                                                                                               |

**【数据清理】**
- `psql -c "DELETE FROM analytics_events WHERE user_id='${U1}' AND created_at > now() - interval '5 minutes'"`

---

## 套件级反模式清单（PR Review 时必查）

下列写法在本套件用例中**视为未通过**：

1. ❌ `composeTestRule.setContent { LoginScreen(viewModel = fakeViewModel) }` — 绕过装配。
2. ❌ `tapOn: index: 0` 类基于 Compose 节点序号的点击 — 真机渲染顺序不稳定。必须用 `id`（testTag）或可见文本定位。
3. ❌ 仅 `assertVisible: "VoiceRoom|语聊房"` 即认定登录成功 — 文案恒真，必须配合 step 3 的 access-log 副作用断言。
4. ❌ Maestro yaml 中硬编码 `appId: com.voiceroom.debug` — 必须用 `${ANDROID_APP_ID}` 注入。
5. ❌ 缺少【数据清理】或清理只删 UI 缓存不清 DB —— 用例之间将互相污染。
6. ❌ **新写 Maestro yaml**（铁律 7）— 本套件所有用例**必须**用 Playwright spec + `@midscene/android` 实现，视觉与交互一律走 `agent.aiTap / aiInput / aiAssert / aiQuery`，不得直接 `adb shell input` 或 Maestro 原语。
