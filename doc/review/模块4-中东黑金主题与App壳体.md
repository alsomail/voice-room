# 全局代码审查报告: 模块 4 - 中东黑金主题与 App 壳体 (MENA Theme & App Shell)
> **当前状态机**：负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]

---

## 0. 流转规则
- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由[GlobalReview]进行全局代码审查
- [GlobalReview]审查通过，则修改负责人 [-] 状态 [✅ Passed]
- [GlobalReview]审查未通过，则修改负责人 [TDD] 状态 [❌ Failed], 并将审查意见填入文档下方
- 处于负责人 [TDD] 状态 [❌ Failed]，则由[TDD]根据审查意见进行代码修复并自测
- [TDD]修复之后，将状态改为负责人 [GlobalReview] 状态 [⏳ In Review]

---

## 1. 审查上下文
- **包含任务**：[模块 4: 中东黑金主题与 App 壳体](../tasks/模块4-中东黑金主题与%20App%20壳体%20(MENA%20Theme%20&%20App%20Shell).md)
  - Android：T-30018 / T-30019 / T-30020 / T-30021 / T-30022 / T-30023 / T-30024 / T-30025 / T-30026
- **关联 TDS**：`doc/tds/android/T-3001{8..26}.md`
- **关联设计文档**：`doc/design/android/T-3001{8..26}.md`
- **产品规范**：`doc/product/android_app_design.md`
- **开始时间**：2026-04-25

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**

审查范围：T-30018 ~ T-30026 共 9 个 Android Task，覆盖 `core/theme/*`、`core/ui/PlaceholderScreen`、`common/ui/OnlineCountBadge`、`feature/main/*`、`feature/splash/*`、`feature/auth/LoginScreen`、`feature/profile/*`、`feature/room/{HallScreen, HallTopBar, RoomCard, RoomScreen, RoomBottomBar, MicSlotsGrid, MicSlotCard}`。

#### 整体亮点
1. `MenaTheme` / `MenaColors` / `MenaShapes` / `MenaTypography` 设计令牌封装清晰，颜色集中并提供 ULong 原始值方便 JVM 单测。
2. `MenaTheme` 对 RTL 通过 `TextUtils.getLayoutDirectionFromLocale` 统一注入 `LocalLayoutDirection`，与 `doc/arch/android/theme.md` 描述一致。
3. `Splash → Main/Login` 使用 `popUpTo("splash") { inclusive = true }` 正确切断回退栈；`SplashViewModel.checkAuth()` re-throw `CancellationException`，结构化并发处理规范。
4. `MainScreen` 的 `NavHost` 使用 `popUpTo(startDestination) { saveState = true } + restoreState = true + launchSingleTop`，符合三 Tab 状态保留约束。
5. `ProfileViewModel` 网络异常降级使用 in-memory 缓存且 re-throw `CancellationException`；退登仅在 ViewModel 清 JWT、UI 通过事件流导航，分层正确。
6. `RoomBottomBar` 麦克风颜色状态机（灰/绿/红 + `enabled=isOnMic`）与 TDS 对齐；退出确认 AlertDialog + 二次确认；🚪 走 `onLeaveRoom` 回调，UI 与业务解耦。
7. `MicSlotCard` 提供 `contentDescription` 三态描述与 `onClickLabel`，无障碍语义到位（配套 `mergeDescendants` 处理）。

#### 缺陷清单

- [ ] **缺陷 1**：[级别 P1] **模块 3 遗留的 `HallScreenVisualConstantsTest` / `OnlineCountBadgeTest` 仍有 9 个用例失败（UInt vs ULong），属于 T-30022 范围必须在本批次修复**
  - **文件与行号**：
    - `app/android/app/src/test/java/com/voice/room/android/feature/room/HallScreenVisualConstantsTest.kt:25-81`
    - `app/android/app/src/test/java/com/voice/room/android/feature/room/OnlineCountBadgeTest.kt:23-42`
    - 实际验证：`./gradlew :app:testDebugUnitTest --tests "...HallScreenVisualConstantsTest" --tests "...OnlineCountBadgeTest"` → `11 tests completed, 9 failed`
    - 失败信息均为：`expected: kotlin.UInt<...> but was: kotlin.ULong<...>`
  - **问题说明**：测试中 `assertEquals(0xFF1A1A2Eu, MenaColors.BACKGROUND_VALUE)` 的字面量后缀 `u` 在 Kotlin 中默认推断为 `UInt`，而 `MenaColors.*_VALUE` 声明为 `ULong`；JUnit `assertEquals(Object, Object)` 因运行时类型不一致直接失败。`HallScreenVisualConstantsTest` 共 7 个用例 100% 失败；`OnlineCountBadgeTest` 4 个用例失败 2 个（颜色相关）。这是 Phase 0.5 视觉升级的"门禁断言"，长期失败等同于"主题色值无法在 CI 上回归保护"，违背模块 3 复审豁免时"必须在 T-30022 内修复"的承诺。
  - **修复建议**：二选一：
    - 推荐：将测试字面量改为 ULong 后缀 `0xFF1A1A2EuL`（或显式 `Color(0xFF1A1A2EuL).value`），保持 `MenaColors.*_VALUE` 为 ULong 的语义（与 `Color(value: ULong)` 构造器对齐）；
    - 或将 `MenaColors.*_VALUE` 改为 `UInt`，再用 `Color(value.toULong() shl 32)` 或 `Color(value.toInt())` 构造（不推荐，会污染常量语义）。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 2**：[级别 P1] **模块 4 大量硬编码中文/阿拉伯文 UI 文案，违反 `doc/architecture/mena_localization.md`「禁止硬编码文案」与模块 2/3 已建立的 `UiText + values-ar` 范式**
  - **文件与行号**（节选，非穷举）：
    - `feature/profile/ProfileContent.kt:153,161,171,183,211,216,233`：`"复制 ID"`/`"💰 ${...} 金币"`/`"缓存"`/`"设置"`/`"退出登录"`
    - `feature/profile/ProfileScreen.kt:91,96,109,117`：`"退出登录"`/`"确认退出当前账号？"`/`"确认"`/`"取消"`
    - `feature/profile/ProfileViewModel.kt:68,70,83`：`"网络异常，显示缓存数据"`/`"加载失败"`/`"ID 已复制"`
    - `feature/main/MessagesPlaceholder.kt:24-25`：`"消息功能即将上线"`/`"敬请期待"`
    - `feature/main/ProfilePlaceholder.kt:27`：`"Me"`（已被替换但仍存在于源码）
    - `feature/auth/LoginScreen.kt:107,113,121,158`：emoji `🎙️` / `"Voice Room"` / `"تسجيل الدخول"`（阿语硬编码）/ 登录按钮文字 `"تسجيل الدخول"`
    - `feature/room/HallScreen.kt:105,118,131,189`：`"加载失败"`/`"重试"`/`"暂无房间"`/`"创建房间"`
    - `feature/room/HallTopBar.kt:39,53,60`：`"VoiceRoom"`/`"榜单"`/`"搜索"`
    - `feature/room/RoomBottomBar.kt:73,122,138,151,166,178,189,190,198,203`：`"说点什么..."`/`"发送"`/`"取消静音"`/`"静音"`/`"礼物"`/`"表情"`/`"退出房间"` 等
    - `feature/room/RoomBottomBar.kt:159`：`Toast.makeText(context, "表情功能敬请期待", ...)`
    - `feature/room/RoomScreen.kt:158,166,170`：`"更多"`/`"榜单"`
    - `feature/room/MicSlotCard.kt:63-65`：`"麦位 ${i+1}，空位，点击上麦"` 等三态描述
    - `feature/splash/SplashScreen.kt:107`：`"App Logo"`
    - `core/theme/AvatarWithFrame.kt:71,81`：`"Avatar"`/`"Default avatar"`
    - 实证：`res/values/strings.xml`/`res/values-ar/strings.xml` 自模块 3 之后**没有新增任何 string 资源**（共 17 条，均为模块 0/2/3 的资源），与新增近 30+ 条 UI 文案完全不匹配。
  - **问题说明**：
    1. 沙特 / 中东市场是 Phase 0.5 的核心交付目标（`product/android_app_design.md` 反复强调），但当前 App 在阿语 Locale 下，Splash/Profile/Hall/Room/BottomBar 等所有 P0 入口仍显示中文，事实上无法本地化交付。
    2. 与模块 2/3 复审通过的 `UiText + values-ar` 契约直接冲突，再次走回模块 1/2 已经被打回过的"硬编码 UI"老路。
    3. `LoginScreen` 反向硬编码阿语 `"تسجيل الدخول"`：英文 Locale 用户也只能看到阿语，对偶问题。
    4. `RoomBottomBar` 的 Toast 在 Composable 内 `Toast.makeText(...)` 直接发，绕过了 `events: SharedFlow<UiText>` → 调用方处理的既定模式（参考 `ProfileScreen` 已正确做了 `events.collect { ShowToast }`），既不可测也不可本地化。
  - **修复建议**：
    1. 将所有 UI 字面量提取到 `res/values/strings.xml`（英文）+ `res/values-ar/strings.xml`（阿语），包括 emoji/品牌名也建议作为 `string` 集中管理（避免重复）。
    2. ViewModel 内统一使用 `UiText.of(R.string.xxx)` 包装事件文案（如 `ProfileEvent.ShowToast(message: UiText)`），UI 层用 `LocalContext.resources` 或 `stringResource` 解析。
    3. `MainTab.labelEn` / `labelAr` 双轨已无需要——直接用 `@StringRes val labelRes: Int` + `stringResource(tab.labelRes)`，让系统按 `values-ar` 自动切换（参见缺陷 3）。
    4. `RoomBottomBar` 的"表情灰禁 Toast" 应改为 `onEmojiClick: () -> Unit` 由调用方处理（或上抛 `UiText` 事件）；不要在 Composable 内 `Toast.makeText`。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 3**：[级别 P1] **`MainTab.labelAr` 是死代码——`MenaBottomNavigation` 仅使用 `labelEn`，三 Tab 文字永远以英文显示，无法跟随系统 Locale 切换为阿语**
  - **文件与行号**：
    - `feature/main/MainTab.kt:20-30`（声明了 `labelEn`/`labelAr`）
    - `feature/main/MenaBottomNavigation.kt:49,55`（icon `contentDescription = tab.labelEn`、Text `tab.labelEn`）
  - **问题说明**：阿语用户在底部三 Tab 上仍看到 "Rooms / Messages / Me"。这等于在产品最显眼的导航上把 RTL 本地化策略架空了。`labelAr` 字段被定义却从未被读取，是典型的"假本地化"。
  - **修复建议**：删除 `labelEn/labelAr` 双字段，改为 `@StringRes val labelRes: Int`（如 `R.string.tab_rooms`），导航栏使用 `stringResource(tab.labelRes)`；values-ar 自动接管。同时把 `contentDescription` 也改为 `stringResource`。
  - **TDD 修复记录**：见缺陷 2 一并处理（`MainTab` 重构为 `@StringRes labelRes: Int`）。

- [ ] **缺陷 4**：[级别 P2] **`MicSlotCard` 硬编码颜色字面量绕过 `MenaColors` 集中管理**
  - **文件与行号**：
    - `feature/room/MicSlotCard.kt:116`：`tint = Color.Red`（禁麦图标）
    - `feature/room/MicSlotCard.kt:205`：`color = Color(0xFF4CAF50).copy(alpha = 0.25f)`（音浪占位）
  - **问题说明**：T-30025 验收标准是"将 RoomScreen 改造为黑金风格，颜色集中在 MenaTheme"。`Color.Red` 与 Material Red 不等于 MenaColors.Error (#E74C3C)；`0xFF4CAF50` 与 MenaColors.Success (#2ECC71) 也是两套绿。色板分裂将直接破坏后续 dark/light 切换或品牌微调。
  - **修复建议**：
    - L116: `tint = MenaColors.Error`
    - L205: `color = MenaColors.Success.copy(alpha = 0.25f)`
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 5**：[级别 P2] **`AvatarWithFrame` 的 `contentDescription` 写死为英文 `"Avatar"` / `"Default avatar"` 且不允许调用方覆盖**
  - **文件与行号**：`core/theme/AvatarWithFrame.kt:71, 81`
  - **问题说明**：作为通用组件被 ProfileScreen / MicSlotCard 等多处调用，但描述写死，不区分语境（如"我的头像"/"用户 Bob 的头像"），无障碍体验差；同时英文硬编码也违反本地化要求。
  - **修复建议**：
    1. 增加 `contentDescription: String? = null` 参数，调用方覆盖；
    2. 默认值改为 `stringResource(R.string.avatar_description)`；
    3. 当 imageUrl 非 null 但调用方不传 cd 时，可用 `Modifier.semantics { invisibleToUser = true }` 让外层 mergeDescendants 接管。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 6**：[级别 P2] **`GoldButton` 文字色 `OnBackground (#FFFFFF)` 在金色渐变 `(#D4AF37 → #FFD700)` 上对比度约 2.5:1，未达 WCAG AA 4.5:1，弱视用户难以辨识**
  - **文件与行号**：`core/theme/GoldButton.kt:63`
  - **问题说明**：作为全站 CTA 主按钮，对比度问题影响所有登录/重试/创建房间等关键操作的无障碍性。设计稿可能写"白字"，但既然已建立 `MenaColors.Background (#1A1A2E)` 这种深色基调，按钮文字使用深色（如 `MenaColors.Background`）将获得 ~7.5:1 的对比度，且更具中东高端金饰审美。
  - **修复建议**：与设计确认后，将文字色改为 `MenaColors.Background`（深色字 on 金底）；若必须保留白字，至少把 enabled=false 的 alpha 0.38 重检（白字 + 38% alpha 几近不可见）。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 7**：[级别 P3 / LOW] **`AvatarWithFrame` 多余 `import androidx.compose.foundation.layout.padding` 未使用 + `clickable` 缺少 `Role`**
  - **文件与行号**：
    - `core/theme/AvatarWithFrame.kt:5`（`import ... padding` unused）
    - `feature/profile/ProfileContent.kt:147,176`（`profile_id_row`/`profile_balance` 的 `clickable` 未指定 `role = Role.Button`，TalkBack 仅按"按钮"提示词不准确）
  - **问题说明**：可清理项；`clickable` 没有 role 在 a11y 上不致命但属于既定 best practice。
  - **修复建议**：删除未用 import；`clickable(role = Role.Button) { ... }` 显式声明角色。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


- [ ] **缺陷 8**：[级别 P3 / LOW] **登录按钮 `onLogin` 未触发 `LoginViewModel.onLogin()`，仅直接 `onLoginSuccess()` 跳转——继承自 T-30002 stub，但 T-30021 视觉升级未顺手补齐**
  - **文件与行号**：`feature/auth/LoginScreen.kt:55-63`（`onLogin = { onLoginSuccess() }`），完全未 `collect(loginViewModel.navEvent)` / 调用 `loginViewModel.onLogin()`
  - **问题说明**：当前点击"登录"按钮：①不调用真实 API；②不保存 JWT；③直接跳到 Main；下次冷启动 Splash 检测无 token，又会回 Login，形成"假登录环"。这条不是 T-30021 引入的回归（commit 历史确认 `aee7c0b` 已经如此），但 T-30021 标题是"视觉升级 + 现有功能不回归"，按理应当顺手补齐 `LaunchedEffect { loginViewModel.navEvent.collect { onLoginSuccess() } } + onLogin = loginViewModel::onLogin`。
  - **修复建议**：
    ```kotlin
    LaunchedEffect(Unit) {
        loginViewModel.navEvent.collect { event ->
            if (event is NavEvent.NavigateToHall) onLoginSuccess()
        }
    }
    LoginScreenContent(..., onLogin = loginViewModel::onLogin)
    ```
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]


#### 复审注意事项（轻提示，非缺陷）

- `MenaTheme` 通过 `LocalConfiguration.locales[0]` 取 Locale 是合理实现，但需确认产品同意"App 内强制深色（始终 darkColorScheme）"——TDS 已声明，记录在案。
- `ProfileViewModel.cachedProfile` 是 in-memory 缓存（ViewModel 生命周期），冷启动后不可用。TDS §6 描述与之一致，但若产品后续要求"飞行模式打开 App 仍能看上次资料"则需切到 DataStore，本批不阻塞。
- `Splash` 总等待 ~1.3s（800ms 动画 + 500ms delay）后才 `checkAuth`。在弱网下 token 检测仍可能失败，目前对失败统一回 Login，行为正确。

**本轮结论**: ❌ 存在 P1 级别问题（3 个 P1 + 3 个 P2 + 2 个 P3）。
*(请在文档头部将状态机修改为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`)*

