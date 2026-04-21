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

### 待开发模块

| 模块 | 当前状态 |
| --- | --- |
| Profile | 🟡 仅保留模块描述与后续落点 |
| Gift / Wallet / Seat / Family / CP / VIP / Backpack / Game | 🔴 仅目录预留，尚无 UI 与逻辑 |

## 二、 当前测试覆盖

- **测试文件**：28 个（含 `test/` 和 `androidTest/` 目录）
- **测试方法**：293 个 `@Test`

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

- Android 端 Auth + Room 大厅 + WS 连接 + 房间核心 + 聊天消息全链路（T-30001 ~ T-30017）已全部落地；大厅页已完成黑金视觉升级（T-30022）；房间页已完成黑金视觉升级（T-30025，HostMicSlot 80dp 金色光圈 + MicSlotCard 副麦 60dp + EmptyMicSlot 虚线"+" + MicSlotsGrid 4列 + ChatMessageList 金色昵称/系统消息金黄，WS/上下麦逻辑不变）；`core/ui/PlaceholderScreen` 通用占位组件与消息Tab占位页（T-30023）已完成，供后续 Profile 等 Tab 复用。
- Gift / Wallet / VIP 等商业化模块尚未展开，仅目录预留。
- 后续开发必须继续对齐 `doc/protocol/` 目录下的对应子文件与服务端广播模型，避免客户端自行推断核心状态。
