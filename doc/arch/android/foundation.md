# Android 核心基建与防腐层骨架

## 一、 分层现状
当前 Android 端已按 `core / common / data / domain / presentation / feature` 建立目录骨架，并优先把第三方依赖隔离在 `core` 层接口之后。

## 二、 基建模块现状

| 模块 | 关键文件 | 当前现状 |
| --- | --- | --- |
| 环境配置 | `core/config/AppEnvironment.kt` | 🟢 负责裁剪环境值，并对物理机 Loopback 地址给出预警 |
| 远程配置 | `core/config/IRemoteConfigService.kt`、`InMemoryRemoteConfigService.kt` | 🟡 只有内存实现，尚未接入远程配置中心 |
| HTTP | `core/network/AppHttpClientFactory.kt`、`NetworkClientConfig.kt` | 🟢 已提供 OkHttp 工厂与超时/重试配置 |
| WebSocket | `core/ws/RoomSocketRequestFactory.kt`、`RoomSocketSession.kt` | 🟢 已完成 WS 请求 URL/Headers 组装；🔴 尚未建立真正连接管理 |
| Telemetry | `core/telemetry/IAnalyticsService.kt`、`NoOpAnalyticsService.kt`、`ICrashReporter.kt`、`NoOpCrashReporter.kt` | 🟡 接口隔离已完成，当前仅 `NoOp` 占位 |
| Media | `core/media/IMediaService.kt`、`NoOpMediaService.kt` | 🟡 防腐层接口已建，RTC 尚未接入 |
| IM | `core/im/IIMService.kt`、`NoOpIMService.kt` | 🟡 防腐层接口已建，IM 尚未接入 |
| Storage / Security / Logging / i18n | `core/*/.gitkeep` 或目录占位 | 🔴 目录已预留，能力未实现 |

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

- WS 长连接生命周期、心跳、重连、鉴权刷新与服务端广播消费。
- 真正的埋点、崩溃上报、媒体与 IM Provider 适配器。
- 本地缓存、安全存储、日志落盘、国际化与 RTL 行为的工程化实现。
