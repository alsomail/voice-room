# Android 启动装配与壳层页面

## 一、 当前启动链路
Android 端已经具备“可启动、可切换、可感知环境”的壳层闭环，核心链路如下：

1. `VoiceRoomApplication` 在启动时创建 `AppContainer`。
2. `AppContainer.fromBuildConfig()` 读取 `BuildConfig` 中的环境参数，并装配基础依赖。
3. `MainActivity` 通过 `viewModels` + `Factory` 注入 `MainViewModel`。
4. `MainViewModel` 根据当前 `MainDestination` 组合 `MainUiState`，页面只做被动渲染。

## 二、 关键文件与职责

| 文件 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/src/main/java/com/voice/room/android/VoiceRoomApplication.kt` | 应用入口，初始化全局容器 | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/common/AppContainer.kt` | 依赖装配、防腐层占位与 Debug 实现注入 | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/presentation/MainActivity.kt` | XML 壳层页面，绑定按钮与文本渲染 | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/presentation/MainViewModel.kt` | 根据目标模块生成 `MainUiState`，记录基础埋点 | 🟢 已落地 |
| `app/src/main/java/com/voice/room/android/presentation/MainUiState.kt` | 页面渲染状态模型 | 🟢 已落地 |
| `app/src/main/res/layout/activity_main.xml` | 首页壳层布局 | 🟢 已落地 |
| `app/src/main/res/values-ar/strings.xml` | 阿拉伯语文案入口 | 🟢 已落地 |

## 三、 当前页面行为

- 默认展示 `Auth Bootstrap`。
- 支持在 `AUTH / ROOM / PROFILE` 三个壳层入口间切换。
- 页面会展示当前 API、WS 地址，以及 Debug Repository 返回的状态文本。
- 若环境配置使用 `localhost / 127.0.0.1 / 0.0.0.0 / ::1`，会提示物理机不可直连。

## 四、 当前限制

- UI 仍是单 Activity + XML 壳层，尚未形成完整导航体系。
- `MainViewModel` 读取的是容器内的 Debug / NoOp 依赖，不代表真实业务链路已打通。
- 页面切换是本地壳层行为，还未接入服务端权威状态、协议回放或弱网恢复流程。
