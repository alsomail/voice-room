# Android 业务骨架与测试现状

## 一、 Feature 现状

### Auth 模块（🟢 已完成，T-30001 ~ T-30004）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 登录页 UI | `feature/auth/LoginScreen.kt` | T-30001 | 🟢 Material3 手机号+验证码登录，RTL 布局支持 |
| 登录 ViewModel | `feature/auth/LoginViewModel.kt` | T-30002 | 🟢 Retrofit 调用登录接口，token 保存 DataStore |
| JWT 拦截器 | `core/network/AuthInterceptor.kt` | T-30003 | 🟢 OkHttp 拦截器自动添加 token，401 跳转登录 |
| 用户信息 Repository | `data/auth/UserRepository.kt` | T-30004 | 🟢 用户信息获取+Room DB 本地缓存+Flow 订阅 |

### Room 大厅模块（🟢 已完成，T-30005 ~ T-30007, T-30022）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 大厅页 UI | `feature/room/LobbyScreen.kt` | T-30005 | 🟢 LazyVerticalGrid + Coil 头像加载 + 在线人数 |
| 房间列表 ViewModel | `feature/room/LobbyViewModel.kt` | T-30006 | 🟢 Paging3 分页加载 + 下拉刷新 + 错误重试 |
| 创建房间对话框 | `feature/room/CreateRoomSheet.kt` | T-30007 | 🟢 BottomSheet 输入房间信息 + 创建成功导航 |
| 大厅页视觉升级 | `feature/room/HallScreen.kt` | T-30022 | 🟢 MenaTheme 黑金风格：RoomCard（深色底+圆角16dp）+ OnlineCountBadge（绿点+人数）+ HallTopBar（金色顶栏）+ CategoryTabRow（分类横滑占位）+ 金色渐变 FAB，Paging3 不变 |

### WebSocket 模块（🟢 已完成，T-30008）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| WS 连接封装 | `core/ws/OkHttpWebSocketClient.kt` | T-30008 | 🟢 指数退避重连 + 心跳保活 + StateFlow 状态 |

### Room 核心模块（🟢 已完成，T-30009 ~ T-30013）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 房间页 UI | `feature/room/RoomScreen.kt` | T-30009 | 🟢 顶部信息 + 麦位 Grid + 聊天列表 + 底部输入栏 |
| 房间 ViewModel | `feature/room/RoomViewModel.kt` | T-30010 | 🟢 JoinRoom/WS 事件监听/离开清理 |
| 麦位组件 | `feature/room/MicSlotCard.kt` | T-30011 | 🟢 三种状态渲染 + Lottie 音浪动画 + RTL |
| 麦克风权限 | `feature/room/MicPermission.kt` | T-30012 | 🟢 Accompanist 运行时权限 + 设置引导 |
| 上麦/下麦逻辑 | `feature/room/MicManager.kt` | T-30013 | 🟢 权限检查 → 上麦请求 → RTC 推流 |

### 房间页视觉升级模块（T-30025）

| 组件 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 主麦位组件 | `feature/room/HostMicSlot.kt` | T-30025 | 🟢 80dp AvatarWithFrame + Canvas GoldGlowRing（stroke 6dp，MenaColors.Primary）居中突出 |
| 副麦位组件 | `feature/room/MicSlotCard.kt` | T-30025 | 🟢 60dp 深色背景，三态 EMPTY/OCCUPIED/MUTED，黑金风格改造 |
| 空麦位组件 | `feature/room/EmptyMicSlot.kt` | T-30025 | 🟢 虚线圆圈 + "+" 图标，可点击触发上麦 onMicSlotClick |
| 麦位网格 | `feature/room/MicSlotsGrid.kt` | T-30025 | 🟢 LazyVerticalGrid 4列（原3列），userScrollEnabled=false |
| 弹幕消息列表 | `feature/room/ChatMessageList.kt` | T-30025 | 🟢 USER_TEXT 昵称金色（MenaColors.Primary）+ SYSTEM_NOTICE 金黄居中 |
| 房间页主屏 | `feature/room/RoomScreen.kt` | T-30025 | 🟢 整体背景 MenaColors.Background 深色，WS/上下麦逻辑不变 |

### 房间底部操作栏升级模块（T-30026）

| 组件 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 底部操作栏 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 Row布局：GoldOutlinedTextField(weight=1f) + MicButton + GiftButton + EmoteButton + ExitButton |
| 麦克风按钮 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 三态：不在麦灰禁 / 在麦未静音绿色 / 在麦已静音红色；点击 toggleMicMute() |
| 礼物按钮 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 灰色禁用样式，点击弹 Toast("敬请期待") |
| 表情按钮 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 灰色禁用样式，点击弹 Toast("敬请期待") |
| 退出按钮 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 点击弹 AlertDialog 二次确认，确认后 leaveRoom() + 导航返回 |
| ViewModel 扩展 | `feature/room/RoomViewModel.kt` | T-30026 | 🟢 新增 toggleMicMute()；RoomUiState 新增 isCurrentUserOnMic / isCurrentUserMuted |

### 房间底部操作栏升级模块（🟢 已完成，T-30026）

| 组件 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 底部操作栏 | `feature/room/RoomBottomBar.kt` | T-30026 | 🟢 Row 布局：GoldOutlinedTextField（输入框）+ MicButton + GiftButton + EmoteButton + ExitButton |
| 麦克风按钮 | `feature/room/RoomBottomBar.kt` → `MicButton` | T-30026 | 🟢 三态：isOnMic=false → 灰色禁用；true+未静音 → 绿色激活；true+已静音 → 红色静音 |
| 礼物/表情按钮 | `feature/room/RoomBottomBar.kt` → `GiftButton`/`EmoteButton` | T-30026 | 🟢 灰色禁用，点击弹 Toast "敬请期待" |
| 退出按钮 | `feature/room/RoomBottomBar.kt` → `ExitButton` | T-30026 | 🟢 点击触发 AlertDialog 二次确认，确认后退出房间 |
| 房间 ViewModel 扩展 | `feature/room/RoomViewModel.kt` | T-30026 | 🟢 新增 toggleMicMute()、isCurrentUserOnMic: StateFlow、isCurrentUserMuted: StateFlow |

> **包路径**：`com.voice.room.android.feature.room`  
> **布局**：`RoomScreen` 的 `Scaffold.bottomBar` 替换为 `RoomBottomBar`，`imePadding()` 保留  
> **状态驱动**：`MicButton` 颜色/可用性完全由 ViewModel 的 `isCurrentUserOnMic`/`isCurrentUserMuted` 驱动，严禁客户端自行推断

### Chat 模块（🟢 已完成，T-30014 ~ T-30017）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 聊天列表 | `feature/room/ChatList.kt` | T-30014 | 🟢 LazyColumn + 自动滚动 + 系统消息居中 |
| 输入框组件 | `feature/room/ChatInput.kt` | T-30015 | 🟢 软键盘适配 + 回车发送 + 空消息禁用 |
| 发送消息逻辑 | `feature/room/ChatSendManager.kt` | T-30016 | 🟢 发送中禁用 + 成功清空 + 失败重试 |
| 接收消息逻辑 | `feature/room/ChatReceiveManager.kt` | T-30017 | 🟢 实时追加 + msg_id 去重 + 自动滚动 |

### Core UI 通用组件（🟢 已完成，T-30023）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 通用占位页 | `core/ui/PlaceholderScreen.kt` | T-30023 | 🟢 接受 icon/title/subtitle 参数的可复用"即将上线"占位组件；`PlaceholderScreenDefaults` 封装设计规范常量（图标 64dp、颜色 OnBackgroundTertiary/Secondary）；testTag `"placeholder_screen"`；供 Messages、Profile 等多 Tab 复用 |

### Messages 消息Tab（🟢 已完成，T-30023）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 消息Tab占位页 | `feature/main/MessagesPlaceholder.kt` | T-30023 | 🟢 委托 `PlaceholderScreen`，显示聊天图标 + "消息功能即将上线" + "敬请期待"；外层 `Box(testTag("messages_placeholder"))` 保持 T-30020 兼容性；18 个测试（7 JVM + 11 androidTest）全部通过 |

### Wallet 钱包模块（🟢 已完成，T-30027，Review R2 通过）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 余额显示 | `feature/wallet/WalletScreen.kt` | T-30027 | 🟢 大卡片展示钻石余额（金色 MenaColors.Primary），充值按钮占位 Toast "即将上线"，testTag `"wallet_balance_value"` |
| 流水列表 | `feature/wallet/WalletTxnPagingSource.kt` + `WalletScreen.kt` | T-30027 | 🟢 Paging3 分页加载（1-based 页码，lastPage 判断：`size < loadSize`），LazyColumn 显示收入/支出项（绿色+/红色-），testTag `"wallet_txn_list"` 挂在 LazyColumn 上 |
| ViewModel | `feature/wallet/WalletViewModel.kt` | T-30027 | 🟢 Manual Factory + StateFlow + SharedFlow；init 调用 `loadBalance` + `subscribeToWsEvents`；WS 按 protocol §6.4.1 读取嵌套 `payload.diamond_balance` 字段（R1-CRITICAL 修复）；401 → NavigateToLogin；CancellationException re-throw |
| 下拉刷新 | `feature/wallet/WalletScreen.kt` | T-30027 | 🟢 PullToRefreshBox 包裹 LazyColumn；刷新时同时更新余额 + 流水首页；401 时发射 NavigateToLogin（R1-HIGH-2 修复） |
| 空状态占位 | `feature/wallet/WalletScreen.kt` | T-30027 | 🟢 LazyColumn itemCount 为 0 时显示占位文案 "暂无流水" + 插画，testTag `"wallet_empty"` |
| 导航集成 | `feature/profile/ProfileContent.kt` + `feature/main/MainScreen.kt` | T-30027 | 🟢 ProfileContent 余额行新增 `onNavigateToWallet` clickable（W27-09）；MainScreen 内部 NavHost 新增 "wallet" composable |
| Data 层 | `data/wallet/WalletApiService.kt` + `WalletTxnPagingSource.kt` + `RetrofitWalletRepository.kt` | T-30027 | 🟢 HTTP API（GET `wallet/balance` + GET `wallet/transactions`）+ Paging3 分页数据源 + Repository 实现（与 RetrofitUserRepository 统一 parseBody 错误处理策略） |
| Domain 层 | `domain/wallet/IWalletRepository.kt` + `WalletTxn.kt` + `TxnsPage.kt` | T-30027 | 🟢 Repository 接口 + 领域模型；`IWalletRepository` 扩展 `getBalance()`/`listTxns()` 接口，保留 `walletPreviewLabel()` 向后兼容 |
| 测试覆盖 | `test/WalletViewModelTest.kt` + `test/WalletTxnPagingSourceTest.kt` | T-30027 | 🟢 22 个单元测试全部通过（WalletViewModelTest 15 个：W27-01~08 + R1-CRITICAL-1/1b + R1-HIGH-3/3b；WalletTxnPagingSourceTest 7 个），Review R2 ✅ |

### Gift 礼物模块（🟢 已完成，T-30028，Review R2 通过）

| 模块 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 礼物面板 | `feature/gift/GiftPanelBottomSheet.kt` | T-30028 | 🟢 ModalBottomSheet（高 55%）+ Tab Row（热门/全部）+ 4列 LazyVerticalGrid，选中项金色边框 |
| 礼物卡片 | `feature/gift/components/GiftCard.kt` | T-30028 | 🟢 可点击的礼物项卡片，展示礼物图标+名称+价格，选中态金色边框 |
| 余额条 | `feature/gift/components/BalanceBar.kt` | T-30028 | 🟢 顶部余额条（💎金额 + 充值按钮占位 Toast"即将上线"），复用 WalletScreen 余额显示，WS BalanceUpdated 实时更新 |
| 数量选择器 | `feature/gift/components/CountSelector.kt` | T-30028 | 🟢 6 个档位 Chip Row（1/10/66/520/786/1314 吉祥数），选中高亮，总价计算 |
| ViewModel | `feature/gift/GiftPanelViewModel.kt` | T-30028 | 🟢 Manual Factory + StateFlow + SharedFlow；loadGifts() 支持 locale 参数（Accept-Language）；selectGift/selectCount/selectRecipient/selectTab/updateRecipients/dismiss/retryLoad 完整业务方法；WS 监听 BalanceUpdated；计算属性：selectedGift/totalPrice/canSend/isBalanceInsufficient/displayGifts |
| UiState 数据类 | `feature/gift/GiftPanelUiState.kt` | T-30028 | 🟢 gifts/loading/error/selectedGiftId/selectedCount/balance/recipients/selectedRecipientId/activeTab，包含 4 个计算属性（selectedGift/totalPrice/canSend/isBalanceInsufficient） |
| 房间集成 | `feature/room/RoomScreen.kt` + `RoomBottomBar.kt` | T-30028 | 🟢 GiftButton 从灰禁 Toast 升级为真实功能；RoomScreen.showGiftPanel 本地状态控制 GiftPanelBottomSheet 显示；传入 onGiftRetry 回调绑定重试逻辑（R1-HIGH 修复） |
| Data 层 | `data/gift/RetrofitGiftRepository.kt` + `data/remote/api/GiftApiService.kt` | T-30028 | 🟢 Repository 实现 60s Mutex 保护内存缓存（R1-MEDIUM 修复），防 TOCTOU 竞态；API 支持 Accept-Language Header；`cacheDurationMs` 作为构造参数方便测试注入 |
| Domain 层 | `domain/gift/IGiftRepository.kt` + `GiftVO.kt` + `MicUserVO.kt` | T-30028 | 🟢 Repository 接口 + 礼物值对象（id/code/name/iconUrl/price/sortOrder/tier）+ 麦位用户值对象（接收者槽） |
| 错误处理 | `feature/gift/GiftPanelBottomSheet.kt` | T-30028 | 🟢 网络失败展示骨架屏占位卡 + "点击重试"按钮，onClick 绑定 onRetry 回调调用 giftViewModel.retryLoad()（G28-09 完整支持） |
| 测试覆盖 | `test/feature/gift/GiftPanelViewModelTest.kt` + `test/data/RetrofitGiftRepositoryTest.kt` | T-30028 | 🟢 GiftPanelViewModelTest 19 个单元测试（G28-02~G28-10 业务验收 + R1-01 重试状态机 + Extra-01~10 边界）；RetrofitGiftRepositoryTest 8 个单元测试（缓存命中/过期/HTTP错误/并发调用单次请求）；336+ tests 全部通过，Review R2 ✅ |

> **包路径**：`com.voice.room.android.feature.gift` / `com.voice.room.android.data.gift`  
> **HTTP API**：`GET /api/v1/gifts/list` + Accept-Language Header（locale 参数从 `LocalConfiguration.locale` 推导）  
> **WS 事件**：订阅 `BalanceUpdated` 信令实时更新余额  
> **集成入口**：`RoomBottomBar.GiftButton` → `onGiftClick { showGiftPanel = true }`  
> **关键设计**：Mutex 缓存 + 错误重试按钮绑定 + 接收者槽占位（T-30029 待接入）



## 二、 当前测试覆盖

- **测试文件**：30 个（含 `test/` 和 `androidTest/` 目录）
- **测试方法**：317 个 `@Test`（新增 T-30027 相关 22 个单元测试）

### 代表性测试文件

| 测试文件 | 覆盖范围 |
| --- | --- |
| `common/AppContainerTest.kt` | 校验容器装配、Debug 依赖注入与 `NoOp` 能力可调用 |
| `core/config/AppEnvironmentTest.kt` | 校验环境值裁剪与物理机 Loopback 警告 |
| `core/network/AppHttpClientFactoryTest.kt` | 校验 OkHttp 超时与重试参数 |
| `core/ws/RoomSocketRequestFactoryTest.kt` | 校验 WS URL 拼接、鉴权头与 OkHttp 兼容转换 |
| `presentation/MainViewModelTest.kt` | 校验默认页面状态、模块切换与基础埋点记录 |
| `androidTest/presentation/MainActivitySmokeTest.kt` | 校验首页可启动且默认标题正确 |

## 三、 对业务推进的含义

- Android 端 Auth + Room 大厅 + WS 连接 + 房间核心 + 聊天消息全链路（T-30001 ~ T-30017）已全部落地；大厅页已完成黑金视觉升级（T-30022）；房间页已完成黑金视觉升级（T-30025，HostMicSlot 80dp 金色光圈 + MicSlotCard 副麦 60dp + EmptyMicSlot 虚线"+" + MicSlotsGrid 4列 + ChatMessageList 金色昵称/系统消息金黄，WS/上下麦逻辑不变）；房间底部操作栏已完成升级（T-30026，RoomBottomBar Row布局：GoldOutlinedTextField输入框 + MicButton三态（不在麦灰禁/在麦绿色/静音红色）+ GiftButton/EmoteButton灰禁Toast + ExitButton AlertDialog二次确认，RoomViewModel新增toggleMicMute()/isCurrentUserOnMic/isCurrentUserMuted）；`core/ui/PlaceholderScreen` 通用占位组件与消息Tab占位页（T-30023）已完成；钱包页完整链路（T-30027，Review R2 通过）已完成：WalletScreen 余额大卡片 + 下拉刷新 + Paging3 流水列表 + 空状态占位，WalletViewModel 初始化加载 + WS 实时更新（按 protocol §6.4.1 读取嵌套 `payload.diamond_balance`）+ 401 导航，Repository 层 HTTP API + Paging3 分页，22 个单元测试全部通过。
- **礼物面板完整链路（T-30028，Review R2 通过）已完成**：GiftPanelBottomSheet ModalBottomSheet（高 55%）+ Tab Row（热门/全部）+ 4列 LazyVerticalGrid（GiftCard 组件，金色边框选中态）+ 顶部 BalanceBar（余额实时 WS 更新）+ CountSelector（6 个吉祥数档位 1/10/66/520/786/1314）+ 接收者槽占位，RetrofitGiftRepository 60s Mutex 缓存防 TOCTOU，支持 Accept-Language 多语言，onRetry 重试按钮绑定 giftViewModel.retryLoad()（R1-HIGH 修复），RoomBottomBar.GiftButton 升级为真实功能，GiftPanelViewModelTest 19+RetrofitGiftRepositoryTest 8 共 27 个新单元测试全部通过，Review R2 ✅。
- **接收者选择器（T-30029）** 与 **SendGift 客户端+幂等（T-30030）** 与 **余额不足引导弹窗（T-30032）** 与 **送礼特效+弹幕（T-30031）** 与 **魅力/财富榜单（T-30033）** 等商业化模块在进行中，依赖礼物面板 T-30028 的完成。
- 后续开发必须继续对齐 `doc/protocol/` 目录下的对应子文件与服务端广播模型，避免客户端自行推断核心状态。
