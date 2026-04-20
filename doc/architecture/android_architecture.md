# 5. Android 架构：Clean Architecture + MVVM

## 5.1 约束

- `feature/*` 只能依赖 `domain` 和 `presentation`。
- `domain` 绝对禁止依赖 Android Framework、Retrofit 或第三方 RTC/IM/埋点 SDK。
- `data` 负责远端数据源（Retrofit）、本地缓存（DataStore / Room）、DTO 到 DomainModel 的转换。
- `ViewModel` 只持有 UI State，不直接操作网络层。
- 房间页 UI 不直接操作 RTC SDK，必须通过 `IMediaService`。
- 埋点、日志、崩溃上报必须通过 `IAnalyticsService` / `ICrashReporter`，不得直接写死具体厂商 SDK。
- UI 层统一使用 **Jetpack Compose** 构建，新页面禁止使用传统 XML 布局。
- 网络层统一使用 **Retrofit 2.11.0**，配合 kotlinx.serialization 或 Moshi 做序列化。
- 本地持久化优先使用 **DataStore 1.1.1**（轻量 KV），复杂结构化数据使用 Room。
- 列表分页统一使用 **Paging3**，配合 `PagingSource` + `LazyColumn` 实现。

## 5.2 Android 关键接口

```kotlin
// 认证与会话
interface IAuthService

// 房间网关与同步
interface IRoomGateway
interface IRoomSyncService

// RTC 媒体防腐层
interface IMediaService

// IM 防腐层
interface IIMService

// 数据仓储
interface IUserRepository
interface IRoomRepository
interface IRoomSnapshotRepository
interface IWalletRepository
interface IGiftRepository

// 观测与崩溃
interface IAnalyticsService
interface ICrashReporter

// 远程配置
interface IRemoteConfigService
```
