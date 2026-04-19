<!-- 
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Android 架构的总路由，严禁在此文件内编写具体的业务逻辑或冗长的代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【二、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【三、当前能力全景与状态】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Android 端架构总索引与状态盘点

## 一、 架构概述
本项目 Android 端当前采用 Kotlin + 分层骨架（`core / common / data / domain / presentation / feature`）推进建设，`ViewModel + MainUiState` 已在壳层页面落地，完整业务级状态管理仍在后续演进中。详情参见全局 `/doc/ARCHITECTURE.md` 的第 5 节。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：请点击以下具体模块查看详细架构说明、API 映射和代码存放路径。
### 实际目录：
- 🧱 [启动装配与壳层页面](./bootstrap.md) - `Application`、`AppContainer`、`MainActivity`、`MainViewModel` 的当前链路。
- 🌐 [核心基建与防腐层骨架](./foundation.md) - 环境配置、HTTP、WebSocket、遥测、媒体、IM 与调试适配器现状。
- 🧩 [业务骨架与测试现状](./features.md) - `auth/room/profile` 能力现状：room 模块已完成大厅页完整链路（T-30005）及 Paging3 无限滚动（T-30006），`profile` 仍为预留；测试覆盖面说明。
- 🔐 [Auth 认证模块](./auth.md) - 登录页组件结构（LoginScreen / LoginViewModel / LoginUiState）、+966 手机号输入、60s 倒计时、RTL 布局支持、StateFlow 数据流。

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发 

### 核心能力
- 🟢 Application 启动装配、`BuildConfig` 环境注入与 `AppContainer` 依赖装配
- 🟢 HTTP 客户端工厂、`RoomSocketRequestFactory` 与物理机 Loopback 预警
- 🟢 `MainActivity`/`MainViewModel` 壳层页面、阿拉伯语资源入口与基础导航切换
- 🟢 `auth` Feature 登录完整链路已完成（T-30001 + T-30002）：LoginScreen / LoginViewModel / IAuthRepository / RetrofitAuthRepository / TokenManager / NavEvent，详见 [auth.md](./auth.md)
- 🟢 Retrofit 2.11.0 HTTP 客户端接入，`RetrofitAuthRepository` 实现登录 / 发码接口调用与错误映射
- 🟢 DataStore 1.1.1 JWT Token 持久化，`TokenManager` 线程安全读写
- 🟢 OkHttp JWT 拦截器（T-30003）：`AuthInterceptor` 自动注入 `Authorization: Bearer` header；`DefaultUnauthorizedHandler` 用 `AtomicBoolean.compareAndSet` 保证并发 401 只处理一次；登录成功后 `resetUnauthorized()` 重置，详见 [auth.md § T-30003](./auth.md#七t-30003-jwt-拦截器)
- 🟢 用户信息 Repository（T-30004）：`IUserRepository.getMe()` 领域接口 + `RetrofitUserRepository` 实现；`UserProfile` 领域模型与 DTO（`UserMeResponseData`）解耦；`coin_balance` 下划线映射、`vipLevel` 默认 0、401 由 `AuthInterceptor` 透传，详见 [auth.md § T-30004](./auth.md#十t-30004-用户信息-repository)
- 🟢 大厅页 UI（T-30005）：`IRoomRepository` + `RetrofitRoomRepository` + `HallViewModel` + `HallScreen`（`LazyVerticalGrid` + Coil + `RoomCard`）；`FakeRoomRepository` 供测试；`AppContainer` 新增 `roomRepository` 属性
- 🟢 房间列表 Paging3 分页（T-30006）：`RoomPagingSource`（`PagingSource<Int, RoomItem>`，`load()` 按页加载，`getRefreshKey()` 标准实现）+ `RoomListViewModel`（`PagingConfig(pageSize=20, initialLoadSize=20, enablePlaceholders=false, prefetchDistance=5)`，`cachedIn(viewModelScope)`）；`HallScreen` 升级为 `collectAsLazyPagingItems()` + Material3 `PullToRefreshBox`（`@OptIn(ExperimentalMaterial3Api::class)`，accompanist 已废弃）；**关键设计决策**：`initialLoadSize = pageSize = 20`，防止 Paging3 默认值 3×pageSize=60 导致 Refresh 与 Append 数据重叠
- 🟢 创建房间对话框（T-30007）：`CreateRoomUiState`（sealed interface：Idle/Loading/Success/Error）+ `CreateRoomViewModel`（输入校验 + `createRoom()` + `resetState()` + Factory）+ `CreateRoomBottomSheet`（Material3 ModalBottomSheet：标题输入框 1-30 字符 Unicode codePointCount 计算、房间类型 RadioButton normal/password/paid、密码框仅 type=password 时显示、`LaunchedEffect(Unit)` 进入时 resetState 防旧 Success 重播）；Data 层新增 `CreateRoomRequest.kt` / `CreateRoomResponseData.kt` DTO；`IRoomRepository` 扩展 `createRoom(title, type, password?): Result<String>`；`RoomApiService` 新增 `@POST("rooms")`；`AppContainer` 为 `roomRetrofit` 注入 `AuthInterceptor`（解决 POST /rooms 401）；**关键设计决策**：`resetState()` 模式防止 BottomSheet 重开时 `LaunchedEffect(uiState)` 重播旧 Success 导航；`MAX_TITLE_LENGTH = 30` 作为 ViewModel `internal const val`，UI 层直接引用避免漂移
- 🟡 `profile` Feature 已有占位描述，Repository / Service 仍为 Debug 实现
- 🟡 Telemetry / Media / IM 已通过接口与 `NoOp*` 适配器隔离第三方依赖
- 🟢 WebSocket 连接封装（T-30008）：`WebSocketState`（sealed class: Connecting/Connected/Disconnected/Error）+ `IWebSocketClient`（接口: connect/disconnect/send/state/messages）+ `OkHttpWebSocketClient`（指数退避重连 1s→60s、30s 心跳保活、`SharedFlow<String>` 消息流）+ `FakeWebSocketClient`（测试辅助），详见 [foundation.md § WebSocket](./foundation.md)
- 🟢 房间页 UI（T-30009）：`RoomUiState`（roomId/roomName/onlineCount/micSlots/messages）+ `MicSlotUi`（index/userId/nickname/avatarUrl/isMuted/isOccupied）+ `ChatMessageUi`（messageId/senderNickname/content/timestamp）；`RoomTopBar`（房间名+在线人数+返回按钮）+ `MicSlotsGrid`（`LazyVerticalGrid` 固定 3 列，高度 240dp，`userScrollEnabled=false`，`testTag("mic_slots_grid")`）+ `MicSlotItem`（空麦/有人/静音三态，Coil AsyncImage 头像，muted icon `testTag`）+ `ChatMessageList`（`LazyColumn` + `weight(1f)` 撑满剩余高度，`LaunchedEffect` 自动滚到底）+ `BottomInputBar`（TextField + 发送按钮）；纯 UI Composable，ViewModel 接入待 T-30010
- 🔴 WebSocket 长连接状态机真实鉴权接入、房间同步、RTC/IM 接入
- 🔴 钱包、礼物、麦位、家族、CP、VIP、背包、小游戏等业务页面与数据链路


### 遗留技术债 (Tech Debt)
- `auth` 模块已完成 UI + ViewModel + Repository + DataStore + OkHttp JWT 拦截器 + 用户信息 Repository 完整链路（T-30001 / T-30002 / T-30003 / T-30004）；`room` 模块已完成大厅页完整链路（T-30005）、Paging3 无限滚动分页（T-30006）、创建房间对话框（T-30007，含 `CreateRoomViewModel` / `CreateRoomBottomSheet` / `IRoomRepository.createRoom` / DTO / `AppContainer` AuthInterceptor 修复）与房间页 UI（T-30009，`RoomScreen` / `RoomUiState` / `MicSlotsGrid(240dp)` / `MicSlotItem` / `ChatMessageList` / `BottomInputBar`，纯 Composable 待 ViewModel 接入）；`core/ws` 已完成 WebSocket 连接封装（T-30008，`WebSocketState` sealed class / `IWebSocketClient` 接口 / `OkHttpWebSocketClient` 指数退避重连+心跳保活 / `FakeWebSocketClient` 测试辅助），房间内 ViewModel（T-30010）等功能仍在 Plan；`AppContainer.fromBuildConfig()` 中 `tokenManager` 为局部变量（debug 占位），引入 DataStore `TokenManager` 时须一并修复（参见 T-30007 TDS 遗留观察）；`profile` 等业务模块仍以 `.gitkeep` 预留，未接入真实 API、WS 协议与服务端广播。
- `core` 层已完成接口隔离，但远程配置、本地存储、安全、日志等能力还只有骨架或占位。
