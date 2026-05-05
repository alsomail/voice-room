# Android 核心基建与防腐层骨架

## 一、 分层现状
当前 Android 端已按 `core / common / data / domain / presentation / feature` 建立目录骨架，并优先把第三方依赖隔离在 `core` 层接口之后。

## 二、 基建模块现状

| 模块 | 关键文件 | 当前现状 |
| --- | --- | --- |
| 环境配置 | `core/config/AppEnvironment.kt` | 🟢 负责裁剪环境值，并对物理机 Loopback 地址给出预警 |
| 远程配置 | `core/config/IRemoteConfigService.kt`、`InMemoryRemoteConfigService.kt` | 🟡 只有内存实现，尚未接入远程配置中心 |
| HTTP | `core/network/AppHttpClientFactory.kt`、`NetworkClientConfig.kt` | 🟢 已提供 OkHttp 工厂与超时/重试配置 |
| HTTP API | Retrofit 2.11.0 + kotlinx.serialization | 🟢 **T-30002 起**已接入，配合 OkHttp 工厂使用 |
| WebSocket | `core/ws/WebSocketState.kt`<br>`core/ws/IWebSocketClient.kt`<br>`core/ws/OkHttpWebSocketClient.kt`<br>`core/ws/FakeWebSocketClient.kt` | 🟢 **T-30008 完成**：`WebSocketState` sealed class（Connecting/Connected/Disconnected/Error）；`IWebSocketClient` 接口（`connect/disconnect/send/state: StateFlow`）；`OkHttpWebSocketClient` 实现指数退避自动重连（1s→2s→4s…60s上限）、30s心跳保活、`SharedFlow<String>` 消息流；`FakeWebSocketClient` 供单元测试注入。<br>**T-30051 完成**：WS 接收链路 5 个关键节点注入可观测性日志（`ws: received` @ `OkHttpWebSocketClient.onMessage` / `ws: parse start\|ok\|failed` & `ws: dispatch` & `rvm: onWsMessage` @ `RoomViewModel` / `ui: chatMessages collected` @ `ChatMessageList`），仅打印 head 80 字符以保护 PII。详见 [TDS T-30051](../../tds/android/T-30051.md) §四 / §六 决策树。 |
| Telemetry | `core/telemetry/IAnalyticsService.kt`、`NoOpAnalyticsService.kt`、`ICrashReporter.kt`、`NoOpCrashReporter.kt` | 🟡 接口隔离已完成，当前仅 `NoOp` 占位 |
| Media | `core/media/IMediaService.kt`、`NoOpMediaService.kt` | 🟡 防腐层接口已建，RTC 尚未接入 |
| IM | `core/im/IIMService.kt`、`NoOpIMService.kt` | 🟡 防腐层接口已建，IM 尚未接入 |
| Storage / Security / Logging / i18n | `core/*/.gitkeep` 或目录占位 | � Storage 已接入 DataStore 1.1.1（T-30002 token 持久化）；其余能力未实现 |

## 三、 调试实现与领域接口

| 层级 | 关键文件 | 当前现状 |
| --- | --- | --- |
| `domain/auth` | `IAuthService.kt` | 🟢 定义接口 |
| `data/auth` | `DebugAuthService.kt` | 🟡 Debug 数据返回 |
| `domain/room` | `IRoomGateway.kt`、`IRoomSyncService.kt` | 🟢 定义接口 |
| `data/room` | `DebugRoomGateway.kt`、`DebugRoomSyncService.kt` | 🟡 Debug 数据返回 |
| `domain/wallet` | `IWalletRepository.kt` | 🟢 定义接口 |
| `data/wallet` | `DebugWalletRepository.kt` | 🟡 Debug 数据返回 |
| `domain/gift` | `IGiftRepository.kt` | 🟢 定义接口 |
| `data/gift` | `DebugGiftRepository.kt` | 🟡 Debug 数据返回 |

## 四、 目前已完成的防腐层约束

- 业务层不直接依赖 RTC、IM、埋点 SDK，而是依赖 `I*Service` 接口。
- `AppContainer` 统一决定当前注入的实现，避免页面层直接 new 第三方能力。
- 环境配置全部通过 `BuildConfig` 注入，未在业务代码中硬编码域名。

## 五、 仍待补全的关键能力

- ~~WS 长连接生命周期、心跳、重连~~ → 已由 T-30008（`OkHttpWebSocketClient`）完成，支持指数退避重连（1s→60s 上限）+ 30s 心跳保活 + `SharedFlow<String>` 消息流。
- ~~鉴权刷新~~ → T-30003（JWT 拦截器）已自动添加 token，401 时跳转登录页。
- 真正的埋点、崩溃上报、媒体与 IM Provider 适配器（当前仅 NoOp）。
- 安全存储（KeyStore）、日志落盘、国际化与 RTL 行为的工程化实现。
