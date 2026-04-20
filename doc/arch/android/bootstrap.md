# Android 启动装配与壳层页面

## 一、 当前启动链路
Android 端已完成从"XML 壳层"到"Compose Navigation"的迁移（T-30019），当前启动链路如下：

1. `VoiceRoomApplication` 在启动时创建 `AppContainer`。
2. `AppContainer.fromBuildConfig()` 读取 `BuildConfig` 中的环境参数，装配基础依赖（`tokenManager` 已提升为公开 `val` 属性）。
3. `MainActivity`（`ComponentActivity`）通过 `setContent { MenaTheme { AppNavGraph(appContainer) } }` 进入 Compose 世界。
4. `AppNavGraph` 创建 `NavHost`，`startDestination = "splash"`，包含 `splash` / `login` / `main` 三条路由。
5. `SplashScreen` 播放 Logo 缩放+淡入动画（800ms EaseOut），完成后 `SplashViewModel.checkAuth()` 检测 JWT：
   - token 非 null 且非空白 → `SplashNavEvent.NavigateToMain` → 导航到 `"main"` 路由
   - token 为 null / 空 / 异常 → `SplashNavEvent.NavigateToLogin` → 导航到 `"login"` 路由
   - 所有导航均使用 `popUpTo("splash") { inclusive = true }` 防止返回键回退到 Splash
6. `"main"` 路由当前为 `MainPlaceholderScreen` 占位（保留旧 `MainViewModel` 展示逻辑），后续 T-30020 将替换为三 Tab 框架。

## 二、 关键文件与职责

| 文件 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/src/main/java/com/voice/room/android/VoiceRoomApplication.kt` | 应用入口，初始化全局容器 | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/common/AppContainer.kt` | 依赖装配、防腐层占位与 Debug 实现注入；`tokenManager: ITokenManager` 公开属性 | 🟢 已落地（T-30019 改造） |
| `app/src/main/java/com/voice/room/android/presentation/MainActivity.kt` | `ComponentActivity` + `setContent { MenaTheme { AppNavGraph } }`，Compose 入口 | 🟢 已落地（T-30019 改造） |
| `app/src/main/java/com/voice/room/android/presentation/AppNavGraph.kt` | Compose Navigation `NavHost`：splash/login/main 三路由骨架 | 🟢 已落地（T-30019 新增） |
| `app/src/main/java/com/voice/room/android/feature/splash/SplashScreen.kt` | Splash 品牌启动页 Composable：Logo 缩放+淡入动画、版本号、testTag 协议 | 🟢 已落地（T-30019 新增） |
| `app/src/main/java/com/voice/room/android/feature/splash/SplashViewModel.kt` | JWT 检测逻辑：`checkAuth()` + `SharedFlow<SplashNavEvent>` + `CancellationException` re-throw | 🟢 已落地（T-30019 新增） |
| `app/src/main/java/com/voice/room/android/feature/splash/SplashNavEvent.kt` | sealed class：`NavigateToMain` / `NavigateToLogin` | 🟢 已落地（T-30019 新增） |
| `app/src/main/res/drawable/ic_logo.xml` | 金色麦克风矢量图标（Splash Logo） | 🟢 已落地（T-30019 新增） |
| `app/src/main/java/com/voice/room/android/presentation/MainViewModel.kt` | 根据目标模块生成 `MainUiState`，记录基础埋点（`MainPlaceholderScreen` 仍引用） | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/presentation/MainUiState.kt` | 页面渲染状态模型 | 🟢 已落地 |
| `app/src/main/res/values-ar/strings.xml` | 阿拉伯语文案入口 | 🟢 已落地 |

## 三、 导航架构

```
AppNavGraph (NavHost)
├── "splash" (startDestination)
│   └── SplashScreen → SplashViewModel.checkAuth()
│       ├── NavigateToMain → popUpTo("splash", inclusive=true) → "main"
│       └── NavigateToLogin → popUpTo("splash", inclusive=true) → "login"
├── "login"
│   └── LoginScreen → onLoginSuccess → popUpTo("login", inclusive=true) → "main"
└── "main"
    └── MainPlaceholderScreen (占位，T-30020 替换为三 Tab 框架)
```

## 四、 当前页面行为

- 启动后首先展示 Splash 页，播放 Logo 动画 ~1.3s（800ms 动画 + 500ms 延迟）。
- JWT 有效 → 直接进入主页；JWT 无效/缺失 → 进入登录页。
- `"main"` 路由展示旧 `MainPlaceholderScreen`，保留当前 API、WS 地址展示与 Debug 状态文本。
- 若环境配置使用 `localhost / 127.0.0.1 / 0.0.0.0 / ::1`，会提示物理机不可直连。

## 五、 测试覆盖

| 测试文件 | 测试类型 | 用例数 | 说明 |
| --- | --- | --- | --- |
| `SplashViewModelTest.kt` | JVM 单元测试 | 6 | SP-01 有效 token、SP-02 null token、SP-03 空白 token、SP-04 异常、SP-05a/SP-05b CancellationException |
| `SplashScreenTest.kt` | androidTest UI 测试 | 4 | SP-06 Logo 可见、SP-07 版本号可见、SP-08 深色背景、SP-09 testTag |

## 六、 当前限制

- `"main"` 路由仍为占位 `MainPlaceholderScreen`，尚未形成三 Tab 框架（T-30020）。
- `MainViewModel` 读取的是容器内的 Debug / NoOp 依赖，不代表真实业务链路已打通。
- `AppContainer.tokenManager` 当前为 debug 匿名实现（`getToken()` 返回 `null`），生产环境需接入 DataStore `TokenManager`。
- 页面切换是本地壳层行为，还未接入服务端权威状态、协议回放或弱网恢复流程。
