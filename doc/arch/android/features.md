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
| 弹幕气泡容器 | `feature/room/ChatMessageList.kt` (UserMessageItem) | T-30052 | 🟢 Surface shape=medium + MenaColors.ChatBubble + widthIn(max=280.dp) + testTag("chat_bubble")，供 Midscene 视觉识别 |
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

### 创建房间表单升级模块（🟢 已完成，T-30036，Review R2 通过）

| 组件 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 主表单页 | `feature/room/CreateRoomScreen.kt` | T-30036 | 🟢 全屏 Scaffold + TopAppBar + 封面预览区 `CoverPreview`（testTag: `cover_preview`，点击触发 `onSelectCover()`）+ 房名输入框 + `CategoryDropdown` + `AnnouncementField` + 密码开关 + `PasswordInputRow`（开关开时显示）+ `GoldButton` 提交按钮（testTag: `btn_submit_create_room`）+ Snackbar 错误提示 |
| 分类下拉 | `feature/room/create/components/CategoryDropdown.kt` | T-30036 | 🟢 `ExposedDropdownMenu` 6 项分类（CHAT/EMOTION/MUSIC/GAME/MATCHMAKING/OTHER），绑定 `RoomCategory` 枚举 |
| 公告输入框 | `feature/room/create/components/AnnouncementField.kt` | T-30036 | 🟢 多行 `TextField` + 字数计数器（当前字数 / 200），超限时计数器文字变红（`MenaColors.Error`）|
| 密码输入行 | `feature/room/create/components/PasswordInputRow.kt` | T-30036 | 🟢 6 位分格输入框，仅接受纯数字，密码 Switch 开启时显示，关闭时隐藏且不传 `password` 字段 |
| 分类枚举 | `feature/room/RoomCategory.kt` | T-30036 | 🟢 `enum class RoomCategory(val key: String, val label: String)` 6 项枚举（CHAT / EMOTION / MUSIC / GAME / MATCHMAKING / OTHER） |
| 表单状态 | `feature/room/CreateRoomFormState.kt` | T-30036 | 🟢 新版 `data class` 含 `canSubmit` 计算属性（基于 `codePointCount` 修复 Unicode 一致性，Review R1 MEDIUM-01 修复）|
| ViewModel 扩展 | `feature/room/create/CreateRoomViewModel.kt` | T-30036 | 🟢 新增 `formState: StateFlow<CreateRoomFormState>` + `updateTitle/updateCoverUrl/updateCategory/updateAnnouncement/togglePasswordEnabled/updatePassword/submit/clearNavigation` 方法；旧版 `uiState` + `createRoom(title,type,password)` 保持不变（向后兼容） |
| Repository 接口升级 | `domain/room/IRoomRepository.kt` | T-30036 | 🟢 `createRoom()` 新增 `coverUrl/category/announcement` 参数（含默认值），修复 Review R1 HIGH-01；`FakeRoomRepository` / `RetrofitRoomRepository` / `HallViewModel.NoOpRoomRepository` 同步更新签名 |
| 请求体 DTO 升级 | `data/remote/model/CreateRoomRequest.kt` | T-30036 | 🟢 新增 `cover_url`、`category`、`announcement` 可选字段，`room_type` 由 `passwordEnabled` 推导（`"password"` or `"normal"`） |
| 测试覆盖 | `test/.../CreateRoomViewModelTest.kt` | T-30036 | 🟢 追加 C36-01~C36-08c 及边界测试共 **38 个测试全部通过**（C36-07c 验证 coverUrl/category/announcement 三字段均传入仓库层）|

#### CreateRoomUiState 字段说明

```kotlin
data class CreateRoomFormState(
    val title: String = "",
    val coverUrl: String = "",           // 从 CoverPickerBottomSheet (T-30037) 回传
    val category: RoomCategory = RoomCategory.CHAT,
    val announcement: String = "",
    val passwordEnabled: Boolean = false,
    val password: String = "",           // 6 位纯数字
    val submitting: Boolean = false,
    val error: String? = null
) {
    val canSubmit: Boolean
        get() = title.codePointCount(0, title.length) in 1..30  // Unicode 字符计数（修复 emoji 边界）
             && announcement.length <= 200
             && (!passwordEnabled || password.matches(Regex("\\d{6}")))
             && coverUrl.isNotEmpty()                            // 封面必选
             && !submitting
}
```

#### canSubmit 校验规则

| 条件 | 规则 | 错误表现 |
|------|------|----------|
| 房间名 | `codePointCount` 在 1~30 字（Unicode 字符，含 emoji 正确计数） | 为空或超限时提交按钮置灰 |
| 封面 | `coverUrl.isNotEmpty()` 必选 | 未选封面时提交按钮永久置灰；封面区 `OutlinedCard` 显示错误色边框 |
| 公告 | ≤200 字（UTF-16 `length`） | 超限时字数计数器变红；提交按钮置灰 |
| 密码 | `passwordEnabled=true` 时 `Regex("\\d{6}")` 6 位纯数字 | 未满足时提交按钮置灰 |
| 密码开关关闭 | 不传 `password` 字段（`ifBlank { null }`） | 密码输入行隐藏，请求体无 `password` |

#### IRoomRepository.createRoom() 新接口

```kotlin
// domain/room/IRoomRepository.kt
interface IRoomRepository {
    suspend fun createRoom(
        title: String,
        type: String,
        password: String? = null,
        coverUrl: String = "",         // 新增 (T-30036)
        category: String = "chat",     // 新增 (T-30036)
        announcement: String? = null   // 新增 (T-30036)
    ): Result<String>  // 返回 roomId
}
```

> **包路径**：`com.voice.room.android.feature.room.create` / `com.voice.room.android.feature.room.create.components`  
> **封面集成**：`CreateRoomScreen` 通过 `onSelectCover` 回调弹出 `CoverPickerBottomSheet`（T-30037），选中后回传 `coverUrl` 调用 `viewModel.updateCoverUrl()`  
> **提交流程**：`submit()` → `CreateRoomRequest` 组装（`room_type` 由 `passwordEnabled` 推导）→ `roomApi.createRoom(req)` → 成功 `navigateToRoom(roomId)` / 失败 `handleError()`（409 活跃房 → Snackbar）  
> **向后兼容**：旧版 `CreateRoomBottomSheet` (T-30007) 与 `CreateRoomViewModel.createRoom(title, type, password)` 接口保持不变，新版通过 `CreateRoomScreen` + `CreateRoomFormState` 并行运行  
> **遗留项**（下一迭代）：`CreateRoomScreen` 标题计数器显示与 `isError` 判断仍用 `title.length`（UTF-16），与 `canSubmit` 的 `codePointCount` 不一致（Review R2 MEDIUM-NEW-01）；`RoomCategory.label` 硬编码中文，待改为 `@StringRes` 支持 i18n（Review R1 LOW-01）

### 房间封面选择器模块（🟢 已完成，T-30037，Review 通过）

| 组件 | 关键文件 | Task | 当前状态 |
| --- | --- | --- | --- |
| 封面选择底部弹窗 | `feature/room/create/CoverPickerBottomSheet.kt` | T-30037 | 🟢 `ModalBottomSheet` 内嵌 `LazyVerticalGrid(Fixed(3))`，3 列展示 8 张中东风预设封面；选中项金色 2dp 边框（`MenaColors.Primary`）+ 角标对钩；确认按钮回传 `coverUrl` 给父页；testTag `cover_picker_sheet` |
| 封面选项常量 | `feature/room/create/CoverOptions.kt` | T-30037 | 🟢 `COVER_OPTIONS: List<CoverOption>` 共 8 项（沙漠 / 清真寺 / 烛灯 / 鹰 / 玫瑰 / 游艇 / 太阳 / 书法），每项含 `drawableRes: Int` + `coverUrl: String`（预设 CDN 路径或 `drawable://` 协议）|
| 封面选项状态 | `feature/room/create/CoverPickerState.kt` | T-30037 | 🟢 `CoverPickerState(selectedIndex: Int)` 由 `mutableStateOf` 驱动，响应式更新选中封面 |
| CreateRoomScreen 集成 | `feature/room/CreateRoomScreen.kt` | T-30037 | 🟢 新增 `showCoverPicker: Boolean` 本地状态（`rememberSaveable`）；`CoverPreview` 点击 → `showCoverPicker = true`；`CoverPickerBottomSheet` onDismiss → `showCoverPicker = false`，onConfirm(url) → `viewModel.updateCoverUrl(url) + showCoverPicker = false` |

#### CoverPickerBottomSheet 架构说明

```kotlin
// feature/room/create/CoverPickerBottomSheet.kt
@Composable
fun CoverPickerBottomSheet(
    currentCoverUrl: String,
    onDismiss: () -> Unit,
    onConfirm: (coverUrl: String) -> Unit
) {
    var state by remember { mutableStateOf(CoverPickerState(
        selectedIndex = COVER_OPTIONS.indexOfFirst { it.coverUrl == currentCoverUrl }.coerceAtLeast(0)
    )) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        LazyVerticalGrid(columns = GridCells.Fixed(3)) {
            itemsIndexed(COVER_OPTIONS) { index, option ->
                CoverOptionItem(
                    option = option,
                    selected = state.selectedIndex == index,
                    onClick = { state = state.copy(selectedIndex = index) },
                    modifier = Modifier.testTag("cover_option_$index")
                )
            }
        }
        GoldButton(
            text = "确认",
            onClick = { onConfirm(COVER_OPTIONS[state.selectedIndex].coverUrl) },
            modifier = Modifier.testTag("btn_confirm_cover")
        )
    }
}
```

#### COVER_OPTIONS 封面清单（8 张中东风格）

| index | 主题 | drawableRes | coverUrl 后缀 |
|-------|------|-------------|--------------|
| 0 | 沙漠 (desert) | `R.drawable.cover_desert` | `cover_desert` |
| 1 | 清真寺 (mosque) | `R.drawable.cover_mosque` | `cover_mosque` |
| 2 | 烛灯 (lantern) | `R.drawable.cover_lantern` | `cover_lantern` |
| 3 | 鹰 (eagle) | `R.drawable.cover_eagle` | `cover_eagle` |
| 4 | 玫瑰 (rose) | `R.drawable.cover_rose` | `cover_rose` |
| 5 | 游艇 (yacht) | `R.drawable.cover_yacht` | `cover_yacht` |
| 6 | 太阳 (sun) | `R.drawable.cover_sun` | `cover_sun` |
| 7 | 书法 (calligraphy) | `R.drawable.cover_calligraphy` | `cover_calligraphy` |

#### testTag 清单

| testTag | 组件 | 用途 |
|---------|------|------|
| `cover_picker_sheet` | `CoverPickerBottomSheet` 根容器 | 断言弹窗已显示 |
| `cover_option_0` ~ `cover_option_7` | 每张封面卡片 | 点击选中指定封面，断言选中态金色边框 |
| `btn_confirm_cover` | 确认按钮 | 点击触发 `onConfirm(coverUrl)` |
| `cover_preview` | `CreateRoomScreen` 封面预览区 | 点击 → `showCoverPicker = true`（T-30036 已定义）|

> **包路径**：`com.voice.room.android.feature.room.create`  
> **状态驱动**：`CoverPickerState.selectedIndex` 由 `mutableStateOf` 驱动，组件内部状态，选中封面通过 `onConfirm` 回调单向流出  
> **集成方式**：`CreateRoomScreen` 通过 `showCoverPicker: Boolean`（`rememberSaveable`）控制弹窗显隐；确认后调用 `viewModel.updateCoverUrl(url)` 写入 `CreateRoomFormState.coverUrl`；`canSubmit` 校验 `coverUrl.isNotEmpty()` 确保必选  
> **内置资源**：8 张 drawable 内置于 `app/src/main/res/drawable/`，MVP 阶段不依赖网络加载

### 密码房进房弹窗（🟢 已完成，T-30038，Review R1 通过）

**最后更新：** 2026-05-24  
**入口点：** `feature/hall/components/PasswordInputDialog.kt`、`feature/hall/HallViewModel.kt`

#### 架构概览

```
HallScreen（房间卡片列表）
  ↓ 点击 has_password=true 的卡片
HallViewModel.openPasswordDialog(roomId)
  ↓ passwordDialogState: StateFlow<PasswordDialogState>
PasswordInputDialog（6 格分格输入 Composable）
  ↓ 用户填完 6 位 → onSubmit(password)
HallViewModel.verifyPassword(roomId, password)
  ↓ HTTP POST /api/v1/rooms/{id}/verify-password
RetrofitRoomRepository.verifyPassword()
  ├─ 成功 → hallEvents.emit(NavigateToRoom(roomId, accessToken))
  ├─ 40103 → PasswordDialogState.Error(remainingAttempts)
  ├─ 42910 → PasswordDialogState.Locked(remainingMinutes)
  └─ 40400 → hallEvents.emit(ShowToast("房间不存在"))
RoomScreen(roomId, accessToken)
  ↓
RoomViewModel.joinRoom(accessToken = token)  // WS JoinRoom 携带 access_token
```

#### 核心组件

| 模块 | 关键文件 | 说明 |
|------|----------|------|
| 弹窗 Composable | `feature/hall/components/PasswordInputDialog.kt` | 6 格 `OutlinedTextField` 分格输入；输满自动 submit；Locked 状态禁用输入框；Verifying 状态仅限输入框 + 提交按钮不可交互 |
| 状态密封类 | `feature/room/PasswordDialogState.kt` | `sealed class PasswordDialogState { Idle / Verifying / Error(remainingAttempts: Int) / Locked(remainingMinutes: Int) }` |
| 大厅事件 | `feature/room/HallEvent.kt` | `sealed class HallEvent { NavigateToRoom(roomId, accessToken) / ShowToast(message) }` |
| ViewModel | `feature/hall/HallViewModel.kt` | `passwordDialogState: StateFlow<PasswordDialogState>`；`hallEvents: SharedFlow<HallEvent>`；方法：`openPasswordDialog` / `verifyPassword` / `dismissPasswordDialog` |
| 领域异常 | `domain/room/PasswordExceptions.kt` | `PasswordWrongException(remainingAttempts)` / `PasswordLockedException(remainingMinutes)` / `RoomNotFoundException` |
| DTO | `data/remote/model/VerifyPasswordModels.kt` | `VerifyPasswordRequest(password)` / `VerifyPasswordResponseData(accessToken)` |
| Repository | `data/room/RetrofitRoomRepository.kt` | `verifyPassword()` 实现；错误码映射 40103→Error / 42910→Locked / 40400→Toast；安全默认值 `remaining_attempts ?: 1`、`remaining_minutes ?: 30` |
| Fake | `data/room/FakeRoomRepository.kt` | `verifyPasswordResult: Result<String>` 可控属性，供单元测试注入 |
| API | `data/remote/api/RoomApiService.kt` | 新增 `suspend fun verifyPassword(roomId: String, body: VerifyPasswordRequest): Response<...>` |
| 接口扩展 | `domain/room/IRoomRepository.kt` | 新增 `suspend fun verifyPassword(roomId: String, password: String): String` |
| RoomViewModel | `feature/room/RoomViewModel.kt` | `joinRoom` 新增 `accessToken: String? = null` 参数，有 token 时 JoinRoom payload 携带 `access_token` 字段 |

#### PasswordDialogState 详解

```kotlin
sealed class PasswordDialogState {
    object Idle : PasswordDialogState()
    object Verifying : PasswordDialogState()
    data class Error(val remainingAttempts: Int) : PasswordDialogState()
    data class Locked(val remainingMinutes: Int) : PasswordDialogState()
}
```

| 状态 | 输入框 | 提交按钮 | 底部文案 | 关闭按钮 |
|------|--------|----------|----------|----------|
| `Idle` | ✅ 可编辑 | ✅ 6位满则启用 | 无 | ✅ 可关闭 |
| `Verifying` | ❌ 禁用 | ❌ 禁用 | 无 | ❌ 禁用 |
| `Error(n)` | ✅ 已清空 | ✅ 重新输入 | 🔴 "密码错误，剩余 N 次" | ✅ 可关闭 |
| `Locked(m)` | ❌ 禁用 | ❌ 禁用 | 🔴 "已被锁定，M 分钟后重试" | ✅ 可关闭 |

> **R1 HIGH 修复**：Locked 状态 `onDismissRequest` 和取消按钮 **不受** `isReadOnly` 限制，仅 `Verifying` 时才屏蔽关闭。`isInputDisabled = state is Verifying || state is Locked` 控制输入框；`onDismissRequest` 独立于输入禁用状态。

#### 错误码映射

| HTTP 错误码 | 含义 | UI 反馈 |
|------------|------|--------|
| `40103` | 密码错误 | `PasswordDialogState.Error(remaining_attempts ?: 1)`；红字显示剩余次数；清空输入框 |
| `42910` | 账号被锁定 | `PasswordDialogState.Locked(remaining_minutes ?: 30)`；输入框禁用；显示剩余分钟 |
| `40400` | 房间不存在 | `HallEvent.ShowToast("房间不存在")`；弹窗关闭 |
| 其他网络错误 | 未知故障 | `HallEvent.ShowToast("网络错误，请重试")` |

**安全默认值**：`remaining_attempts ?: 1`、`remaining_minutes ?: 30`（防止服务端未传字段导致 NPE 或 0 次/0 分钟的错误提示）

#### HallViewModel 密码弹窗状态流

```kotlin
// HallViewModel.kt
val passwordDialogState: StateFlow<PasswordDialogState> =
    _passwordDialogState.asStateFlow()        // 初始 Idle

val hallEvents: SharedFlow<HallEvent> =
    _hallEvents.asSharedFlow()                // 一次性导航/Toast 事件

fun openPasswordDialog(roomId: String)        // has_password=true 卡片点击触发
fun verifyPassword(roomId: String, password: String)   // 提交 6 位密码
fun dismissPasswordDialog()                   // 返回键 / 取消按钮触发
```

状态流转路径：
```
Idle ──openPasswordDialog──► Idle (弹窗显示)
         用户输完 6 位
         ──verifyPassword──► Verifying
                             ├─成功──► Idle + NavigateToRoom 事件
                             ├─40103──► Error(n)  → 用户重新输入 → Verifying ...
                             ├─42910──► Locked(m)
                             └─40400──► Idle + ShowToast 事件
任意状态 ──dismissPasswordDialog──► Idle
```

#### testTag 清单

| testTag | 组件 | 用途 |
|---------|------|------|
| `password_dialog` | `PasswordInputDialog` 根容器 | 断言弹窗已显示 |
| `password_input` | 6 格输入区域整体 | 查找整体输入区 |
| `password_digit_0` ~ `password_digit_5` | 各格 `OutlinedTextField` | 断言焦点跳转 / 输入内容 |
| `btn_submit_password` | 提交按钮 | 断言禁用/启用态，触发提交 |
| `password_error_text` | 底部红色错误/锁定文案 | 断言错误次数文案 / 锁定提示文案 |

> **包路径**：`com.voice.room.android.feature.hall.components`（PasswordInputDialog）、`com.voice.room.android.feature.room`（PasswordDialogState、HallEvent）  
> **测试文件**：`test/.../HallPasswordDialogTest.kt`（P38-01~P38-06b）、`test/.../RetrofitRoomRepositoryTest.kt`（VP01/VP02 fallback 值）  
> **服务端协议**：对齐 `doc/arch/server/room.md` §八~§十二（POST `/api/v1/rooms/:id/verify-password`，Redis `pwd_fail`/`pwd_lock` Key，5 次锁定流程）

---

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



### 观众席底部弹窗（🟢 已完成，T-30039，Review R2 通过）

**最后更新：** 2026-05-25  
**入口点：** `feature/room/components/AudienceBottomSheet.kt`、`feature/room/RoomViewModel.kt`

#### 架构概览

```
RoomScreen（房间主页）
  ↓ 点击"观众席"入口
RoomViewModel.audienceState: StateFlow<AudienceUiState>
  ↓
AudienceBottomSheet（ModalBottomSheet，占屏 70%）
  ├─ Header "观众席 ($total)"
  └─ LazyColumn
       ├─ Section "麦上 (N)"  → onMic 列表（testTag: audience_header_on_mic）
       │    └─ MemberRow × N
       └─ Section "观众 (N)"  → audience 列表（testTag: audience_header_observers）
            └─ MemberRow × M
  ↓ 滚到底触发 loadMoreMembers()
RoomViewModel.loadMoreMembers()
  ↓ hasMore=true 时
IRoomMemberRepository.listMembers(roomId, page, limit=20)
  ↓
RetrofitRoomApi GET /api/v1/rooms/{roomId}/members?page=N&limit=20
```

#### 核心组件

| 模块 | 关键文件 | 说明 |
|------|----------|------|
| 底部弹窗 | `feature/room/components/AudienceBottomSheet.kt` | `ModalBottomSheet` 70% 高；LazyColumn 分页；testTag 完整 |
| 成员行 UI | `feature/room/components/MemberRow.kt` | 头像（Coil）+ 昵称 + 角色徽章（👑 owner / 🛡️ admin）+ 在线时长 |
| UI 状态 | `feature/room/RoomUiState.kt` | 新增 `AudienceUiState` data class |
| 数据 DTO | `data/model/RoomMember.kt` | `id/nickname/avatarUrl/role/slot/joinedAt/micMuted/chatMuted` |
| 仓库接口 | `data/room/IRoomMemberRepository.kt` | `listMembers()` + `MemberListResult` + `NoOpRoomMemberRepository` |
| Retrofit API | `data/api/RoomApi.kt` | `listMembers(roomId, page, limit)` GET 接口 |
| ViewModel 扩展 | `feature/room/RoomViewModel.kt` | 新增 `audienceState`/`selectedMember` StateFlow + `loadMoreMembers()`/`onMemberClick()` + WS 事件处理 |
| 测试 | `test/.../AudienceViewModelTest.kt` | A39-01~07 + extra-01~05 + A39-first-page，共 13 个全部通过 |
| Fake 仓库 | `data/room/FakeRoomMemberRepository.kt` | 测试辅助，可注入 page 响应 |

#### AudienceUiState 详解

```kotlin
// feature/room/RoomUiState.kt
data class AudienceUiState(
    val onMic: List<RoomMember> = emptyList(),
    val audience: List<RoomMember> = emptyList(),
    val total: Int = 0,
    val loading: Boolean = false,
    val currentPage: Int = 0,  // 初始值为 0，表示"尚未加载任何页"；
                                // 首次 loadMoreMembers() 计算 nextPage = 0+1 = 1，
                                // 正确请求 API 第 1 页（API 1-indexed）。
                                // Review R1 HIGH-01 修复：原始值 1 → 0。
    val hasMore: Boolean = true
)
```

> **`currentPage` 初始值为 0 的原因**：API 使用 1-indexed 页码，`loadMoreMembers()` 每次以 `nextPage = currentPage + 1` 发起请求。若初始值为 1，首次调用会跳过第 1 页直接请求第 2 页，导致进入房间时所有现有成员不可见（静默数据缺失 Bug，Review R1 HIGH-01）。初始值 0 确保首次请求 `page=1`，用回归测试 `A39-first-page` 锁定。

#### IRoomMemberRepository + listMembers API

```kotlin
// data/room/IRoomMemberRepository.kt
interface IRoomMemberRepository {
    /**
     * 获取房间成员分页列表。
     * @param roomId  目标房间 ID
     * @param page    1-indexed 页码；首页传 1
     * @param limit   每页条数，默认 20
     */
    suspend fun listMembers(
        roomId: String,
        page: Int,
        limit: Int = 20
    ): MemberListResult
}

data class MemberListResult(
    val onMic: List<RoomMember>,
    val audience: List<RoomMember>,
    val total: Int,
    val hasMore: Boolean   // false 时 loadMoreMembers() 停止继续加载
)

/** NoOp 实现供未接 DI 的构造场景使用，hasMore=false 防止无界分页（R1 HIGH-02 修复） */
class NoOpRoomMemberRepository : IRoomMemberRepository {
    override suspend fun listMembers(roomId: String, page: Int, limit: Int) =
        MemberListResult(emptyList(), emptyList(), 0, hasMore = false)
}
```

#### 分页规则

| 条件 | 行为 |
|------|------|
| `hasMore = true` | 滚到底时触发 `loadMoreMembers()`，请求 `page = currentPage + 1` |
| `hasMore = false` | **停止加载**，不再发起网络请求；`NoOpRoomMemberRepository` 固定返回 `false` 防无界分页 |
| `loading = true` | 防重复请求（`loadMoreMembers()` 进入时检查） |

#### WS 事件映射

| WS 事件 | 处理逻辑 |
|---------|---------|
| `UserJoined` | 将新用户追加到 `audience` 列表尾部；`total + 1` |
| `UserLeft` | 从 `onMic` 或 `audience` 中移除对应 `userId`；`total - 1` |
| `MicTaken` | 将用户从 `audience` 移入 `onMic`（填充 `slot` 字段）；兜底：用户未经 `UserJoined` 直接上麦时仅含 `id+nickname`（FIXME：缺 `role`/`avatarUrl`，MEDIUM-01 遗留）|
| `MicLeft` | 将用户从 `onMic` 移回 `audience`，清除 `slot` |
| `AdminChanged` | 更新对应用户的 `role` 字段（`admin` ↔ `member`），含 `previous_admin_id` 恢复前任角色 |

#### testTag 清单

| testTag | 组件 | 用途 |
|---------|------|------|
| `audience_sheet` | `AudienceBottomSheet` 根容器（`Key('audience_sheet')`） | 断言弹窗已显示 |
| `audience_item_${userId}` | 每行 `MemberRow`（`Key('audience_item_$userId')`） | 点击触发 `onMemberClick`；断言成员存在 |
| `audience_header_on_mic` | "麦上 (N)" 分组标头 | 断言麦上分区渲染 |
| `audience_header_observers` | "观众 (N)" 分组标头 | 断言观众分区渲染 |

> **包路径**：`com.voice.room.android.feature.room.components`（AudienceBottomSheet、MemberRow）、`com.voice.room.android.data.room`（IRoomMemberRepository、FakeRoomMemberRepository）  
> **点击回调**：`MemberRow` 点击 → `RoomViewModel.onMemberClick(member)` → `selectedMember` 更新 → `RoomScreen` 弹出 `UserActionBottomSheet`（T-30040）  
> **性能**：LazyColumn 使用稳定 key（`userId`），100 人房间滚动不卡顿；头像由 Coil 缓存  
> **服务端协议**：对齐 `doc/arch/server/room.md` §十三~§十五（GET `/api/v1/rooms/:id/members`，角色优先级 owner>admin>member，1 次批量 SQL）  
> **遗留项**：[MEDIUM-01] `MicTaken` 兜底 `RoomMember` 缺 `role`/`avatarUrl`，已加 FIXME；[MEDIUM-02] A39-01「双空状态文案」Compose UI 层断言待补充；[LOW-01] `RoomViewModel` KDoc WS 映射表待补全

---

### 用户操作菜单 BottomSheet（🟢 已完成，T-30040，Review 通过）

**最后更新**：2026-05-26  
**包路径**：`com.voice.room.android.feature.room.governance`  
**入口**：`RoomScreen` 内 `selectedMember` 非空时弹出（来自 T-30039 `MemberRow` 点击回调）

#### 架构概览

```
RoomViewModel
  ├── selectedMember: StateFlow<RoomMember?>
  │     └── onMemberClick(member) → selectedMember = member
  ├── pendingRevokeTarget: RoomMember?   // RevokeAdmin 两步确认中间态
  ├── selectedKickTarget: RoomMember?    // 联动 T-30041 KickReasonDialog
  └── onActionSelected(action, target)
        ├── REVOKE_ADMIN → pendingRevokeTarget = target
        │     → emit UserActionEvent.ShowRevokeAdminConfirm(nickname)
        ├── KICK → selectedKickTarget = target
        │     → emit UserActionEvent.ShowKickReasonDialog
        └── 其余动作 → 直接发 WS 信令

RoomScreen
  └── selectedMember != null
        → UserActionBottomSheet(member, myRole)
              └── ActionMatrix.computeActions(myRole, targetRole)
                    → List<UserAction>（按角色组合过滤，不可用项不渲染）
```

#### Role 枚举 + UserAction 枚举

```kotlin
enum class Role { OWNER, ADMIN, MEMBER }

enum class UserAction {
    INVITE_MIC,    // 抱上麦（owner/admin → member）
    MUTE_MIC,      // 禁麦（owner → admin/member；admin → member）
    MUTE_CHAT,     // 禁言（同上）
    KICK,          // 踢出（→ T-30041 KickReasonDialog）
    ASSIGN_ADMIN,  // 任命管理员（owner → member）
    REVOKE_ADMIN,  // 卸任管理员（owner → admin，两步确认）
    VIEW_PROFILE,  // 查看资料（占位，全角色可见）
    REPORT,        // 举报（全角色可见）
}
```

#### ActionMatrix.kt 权限矩阵（computeActions，9 角色组合）

```kotlin
object ActionMatrix {
    /**
     * 根据（操作者角色, 目标角色）计算可用操作列表。
     * 覆盖 9 种合法组合；不可用项不渲染。
     *
     * @param myRole     当前登录用户在房间内的角色
     * @param targetRole 被点击用户在房间内的角色
     * @return 有序操作列表（功能项在前，VIEW_PROFILE/REPORT 在后）
     */
    fun computeActions(myRole: Role, targetRole: Role): List<UserAction> = when {
        // ── 操作者: OWNER ──────────────────────────────────────────────
        myRole == Role.OWNER && targetRole == Role.ADMIN ->
            listOf(REVOKE_ADMIN, MUTE_MIC, MUTE_CHAT, KICK, VIEW_PROFILE, REPORT)
        myRole == Role.OWNER && targetRole == Role.MEMBER ->
            listOf(INVITE_MIC, ASSIGN_ADMIN, MUTE_MIC, MUTE_CHAT, KICK, VIEW_PROFILE, REPORT)
        myRole == Role.OWNER && targetRole == Role.OWNER ->
            listOf(VIEW_PROFILE)                          // 自己看自己（理论上不触发）

        // ── 操作者: ADMIN ──────────────────────────────────────────────
        myRole == Role.ADMIN && targetRole == Role.OWNER ->
            listOf(VIEW_PROFILE, REPORT)                  // 管理员不能操作房主
        myRole == Role.ADMIN && targetRole == Role.ADMIN ->
            listOf(VIEW_PROFILE, REPORT)                  // 管理员不能操作同级
        myRole == Role.ADMIN && targetRole == Role.MEMBER ->
            listOf(INVITE_MIC, MUTE_MIC, MUTE_CHAT, KICK, VIEW_PROFILE, REPORT)

        // ── 操作者: MEMBER ─────────────────────────────────────────────
        myRole == Role.MEMBER && targetRole == Role.OWNER ->
            listOf(VIEW_PROFILE, REPORT)
        myRole == Role.MEMBER && targetRole == Role.ADMIN ->
            listOf(VIEW_PROFILE, REPORT)
        myRole == Role.MEMBER && targetRole == Role.MEMBER ->
            listOf(VIEW_PROFILE, REPORT)

        else -> listOf(VIEW_PROFILE, REPORT)
    }
}
```

#### UserActionBottomSheet testTag 清单

| testTag | 组件 | 用途 |
|---------|------|------|
| `user_action_sheet` | `UserActionBottomSheet` 根容器 | 断言弹窗已显示 |
| `user_action_INVITE_MIC` | 抱上麦 菜单项（`Key('user_action_INVITE_MIC')`） | 断言按权限可见 / 触发 ForceTakeMic WS |
| `user_action_MUTE_MIC` | 禁麦 菜单项 | 断言按权限可见 / 触发 MuteUser(type=mic) WS |
| `user_action_MUTE_CHAT` | 禁言 菜单项 | 断言按权限可见 / 触发 MuteUser(type=chat) WS |
| `user_action_KICK` | 踢出 菜单项 | 点击 → `selectedKickTarget` 更新 → 弹 T-30041 KickReasonDialog |
| `user_action_ASSIGN_ADMIN` | 任命管理员 菜单项 | 触发 AssignAdmin AlertDialog 确认 |
| `user_action_REVOKE_ADMIN` | 卸任管理员 菜单项 | 触发两步确认流程（pendingRevokeTarget） |
| `user_action_VIEW_PROFILE` | 查看资料（占位） | Toast "功能开发中"，全角色可见 |
| `user_action_REPORT` | 举报 菜单项 | Toast "已举报"，全角色可见 |
| `btn_confirm_revoke_admin` | 卸任确认 AlertDialog 确认按钮 | 断言可见 / 触发 `confirmRevokeAdmin()` → WS TransferAdmin(revoke) |

#### RevokeAdmin 两步确认流程

```
用户点击 [卸任管理员]
  → RoomViewModel.onActionSelected(REVOKE_ADMIN, targetMember)
      → pendingRevokeTarget = targetMember
      → emit UserActionEvent.ShowRevokeAdminConfirm(targetMember.nickname)

RoomScreen 收到 ShowRevokeAdminConfirm
  → AlertDialog（标题"确认卸任 ${nickname} 的管理员？"）
      [确认]（Key('btn_confirm_revoke_admin')）
        → RoomViewModel.confirmRevokeAdmin()
            → WS TransferAdmin { action = "revoke", target_user_id = ... }
            → 等待 AdminChanged 广播
            → AdminChanged 到达
                → updateMemberRole(previous_admin_id, MEMBER)
                → pendingRevokeTarget = null
                → dismiss AlertDialog + dismiss UserActionBottomSheet
      [取消]
        → pendingRevokeTarget = null → dismiss dialog
```

#### 与 T-30041 的联动（selectedKickTarget）

- 用户点击 **踢出** → `RoomViewModel.selectedKickTarget = targetMember`；`RoomScreen` 监听非空时弹出 `KickReasonDialog`（T-30041）  
- `KickReasonDialog` 确认后调用 `RoomViewModel.kickUser(reason)` → WS `KickUser` 信令  
- WS `UserKicked` 广播回来 → `selectedKickTarget = null` + dismiss KickReasonDialog + dismiss `UserActionBottomSheet`  
- `selectedKickTarget` 为独立字段，与 `selectedMember` 解耦，避免踢出流程中意外关闭观众席列表

> **包路径**：`com.voice.room.android.feature.room.governance`（UserActionBottomSheet、ActionMatrix、UserAction、Role）  
> **状态持有**：`RoomViewModel.selectedMember`（来自 T-30039）、`pendingRevokeTarget`、`selectedKickTarget`（对接 T-30041）  
> **权限对齐**：严格对应 `doc/product/phase1_room_governance.md §2.3` 权限矩阵与服务端 `doc/arch/server/room.md` §三十二~三十九（TransferAdmin/KickUser/MuteUser 信令）  
> **遗留项**：[LOW-01] `INVITE_MIC` 对应的 `ForceTakeMic` WS 发送逻辑待 T-30044 补全；[LOW-02] `MUTE_MIC`/`MUTE_CHAT` 服务端交互亦待 T-30044 集成；[LOW-03] `ASSIGN_ADMIN` 确认 Dialog testTag `btn_confirm_assign_admin` 待 T-30043 联调验证

---

### 踢人原因选择弹窗（🟢 已完成，T-30041，Review 通过）

**最后更新**：2026-05-27  
**包路径**：`com.voice.room.android.feature.room.governance`  
**入口**：`RoomViewModel.selectedKickTarget` 非空时，`RoomScreen` 弹出 `KickReasonDialog`（由 T-30040 用户操作菜单点击「踢出」触发）

#### 架构概览

```
RoomViewModel
  ├── selectedKickTarget: RoomMember?      // T-30040 写入；T-30041 读取
  └── kickUser(reason: String)
        → WS KickUser { target_user_id, reason }
        → 等待 UserKicked / UserLeft / MicLeft 广播
        → 广播到达 → selectedKickTarget = null + dismiss KickReasonDialog

RoomScreen
  └── selectedKickTarget != null
        → KickReasonDialog(
              target     = selectedKickTarget,
              onConfirm  = { reason → viewModel.kickUser(reason) },
              onDismiss  = { viewModel.clearKickTarget() }
           )
```

#### KickReason 枚举

```kotlin
// feature/room/governance/KickReason.kt
enum class KickReason(val label: String) {
    HARASSMENT("骚扰"),   // 默认选中
    SPAM("刷屏"),
    ABUSE("辱骂"),
    OTHER("其他"),        // 选中时必须填写自定义文本
}
```

#### KickDialogState（canSubmit 逻辑）

```kotlin
// feature/room/governance/KickDialogState.kt
data class KickDialogState(
    val selectedReason: KickReason = KickReason.HARASSMENT,   // 默认"骚扰"
    val customText: String = "",                               // 仅 OTHER 时启用
    val isSubmitting: Boolean = false,
) {
    /**
     * 提交按钮可用条件：
     *   非 OTHER → 始终可提交（已有预设原因）
     *   OTHER    → customText 去空格后非空才可提交
     * isSubmitting = true 时置灰防重复点击。
     */
    val canSubmit: Boolean
        get() = !isSubmitting && when (selectedReason) {
            KickReason.OTHER -> customText.isNotBlank()
            else             -> true
        }
}
```

#### KickReasonDialog 组件（AlertDialog，dismissOnClickOutside=false）

```kotlin
// feature/room/governance/KickReasonDialog.kt
@Composable
fun KickReasonDialog(
    target: RoomMember,
    state: KickDialogState,
    onReasonSelected: (KickReason) -> Unit,
    onCustomTextChange: (String) -> Unit,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        // dismissOnClickOutside=false：踢人操作不允许误触关闭
        properties = DialogProperties(dismissOnClickOutside = false),
        onDismissRequest = { /* 禁止背景点击关闭；仅 [取消] 按钮可关闭 */ },
        title   = { Text("踢出 ${target.nickname}") },
        text    = {
            Column {
                KickReason.entries.forEachIndexed { index, reason ->
                    Row(
                        modifier = Modifier
                            .testTag("kick_reason_$index")   // Key('kick_reason_$index')
                            .clickable { onReasonSelected(reason) },
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(
                            selected = state.selectedReason == reason,
                            onClick  = { onReasonSelected(reason) },
                        )
                        Text(reason.label)
                    }
                }
                // 自定义文本框：仅 OTHER 时可见
                if (state.selectedReason == KickReason.OTHER) {
                    OutlinedTextField(
                        value         = state.customText,
                        onValueChange = onCustomTextChange,
                        modifier      = Modifier.testTag("kick_reason_custom_input"),
                        placeholder   = { Text("请填写原因（必填）") },
                        singleLine    = true,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick  = onConfirm,
                enabled  = state.canSubmit,
                modifier = Modifier.testTag("btn_confirm_kick"),   // Key('btn_confirm_kick')
            ) { Text("确认踢出") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消") }
        },
    )
}
```

#### reason 字段转义逻辑（JSON 安全）

```kotlin
// feature/room/RoomViewModel.kt
fun kickUser(reason: String? = null) {
    val target = selectedKickTarget ?: return
    viewModelScope.launch {
        _kickDialogState.update { it.copy(isSubmitting = true) }

        // reason 字段转义：去除首尾空白，替换双引号为全角引号，
        // 防止自定义文本注入 JSON KickUser 信令字符串。
        val safeReason: String? = reason
            ?.trim()
            ?.replace("\"", "\u201C")   // " → "（全角左引号）
            ?.replace("\\", "\\\\")     // 反斜杠转义

        wsClient.send(KickUserPayload(
            target_user_id = target.id,
            reason         = safeReason,
        ))
    }
}
```

> **转义规则说明**：  
> - `customText` 由用户自由输入，必须在发 WS 信令前完成转义，避免 `"` 截断 JSON payload。  
> - 采用全角替换（而非反斜杠转义）以兼容服务端对 `reason` 的字符串日志存储，不引入额外解析复杂度。  
> - 预设 `HARASSMENT`/`SPAM`/`ABUSE` 枚举值只传枚举名（英文大写），无需额外转义。

#### 与 T-30040 的联动（selectedKickTarget）

```
UserActionBottomSheet [踢出] 按钮
  → RoomViewModel.onActionSelected(KICK, targetMember)
      → selectedKickTarget = targetMember
      → emit UserActionEvent.ShowKickReasonDialog

RoomScreen 收到 ShowKickReasonDialog（或监听 selectedKickTarget != null）
  → KickReasonDialog 弹出（AlertDialog，dismissOnClickOutside=false）

用户选择原因 → [确认踢出]（btn_confirm_kick）
  → KickDialogState.canSubmit 检查通过
  → RoomViewModel.kickUser(safeReason)
      → WS KickUser { target_user_id, reason }

WS 服务端广播 UserKicked / UserLeft / MicLeft
  → selectedKickTarget = null           ← 关闭 KickReasonDialog
  → AudienceUiState 移除对应用户        ← 刷新观众席
  → UserActionBottomSheet dismiss        ← 关闭操作菜单
  → Toast("已踢出")
```

- `selectedKickTarget` 为独立字段，与 `selectedMember` 完全解耦：踢出流程进行中，观众席弹窗（T-30039）和操作菜单（T-30040）的展示状态不受干扰。  
- 踢出失败（WS error 事件）→ `isSubmitting = false`，Toast 展示服务端错误原因，弹窗保持打开。

#### testTag 清单

| testTag | 组件 | 用途 |
|---------|------|------|
| `kick_reason_0` | 骚扰（HARASSMENT）RadioButton 行（`Key('kick_reason_0')`） | 断言默认选中；点击切换选项 |
| `kick_reason_1` | 刷屏（SPAM）RadioButton 行（`Key('kick_reason_1')`） | 断言可点击切换 |
| `kick_reason_2` | 辱骂（ABUSE）RadioButton 行（`Key('kick_reason_2')`） | 断言可点击切换 |
| `kick_reason_3` | 其他（OTHER）RadioButton 行（`Key('kick_reason_3')`） | 选中后断言 `kick_reason_custom_input` 出现 |
| `kick_reason_custom_input` | OTHER 自定义输入框（`OutlinedTextField`） | 断言仅 OTHER 时可见；输入后 `btn_confirm_kick` 变可用 |
| `btn_confirm_kick` | 确认踢出按钮（`Key('btn_confirm_kick')`） | 断言 canSubmit=false 时 `isEnabled=false`；点击触发 `kickUser()` |

> **包路径**：`com.voice.room.android.feature.room.governance`（KickReasonDialog、KickReason、KickDialogState）  
> **状态持有**：`RoomViewModel.selectedKickTarget`（来自 T-30040）、`RoomViewModel._kickDialogState: MutableStateFlow<KickDialogState>`  
> **服务端协议**：对齐 `doc/arch/server/room.md` §十六~§二十三（KickUser 信令格式、权限矩阵、10min 冷却 Redis Key）  
> **遗留项**：[LOW-01] 踢出冷却期结束后（TTL 600s）UI 层无需处理，服务端 JoinRoom 42911 校验兜底；[LOW-02] reason 字段服务端目前仅存日志，不做校验，后续可加枚举约束

---

## T-30042 被踢/被禁提示弹窗

### 概述
当 WS 收到 `UserKicked` / `UserMuted` 信令时，向用户展示全屏确认弹窗，提供 Cooldown 拦截保护。

### 组件结构
| 组件 | 职责 |
|------|------|
| `UserKickedDialog` | 全屏弹窗，展示踢出原因和冷却时间，dismissOnClickOutside=false |
| `MuteStatusChip` | 禁麦/禁言倒计时 Chip，嵌入房间状态栏 |
| `MuteCountdownViewModel` | 管理 mic/chat 独立倒计时，注入 Clock 接口 |
| `KickCooldownStore` | 保存踢出记录，Application 单例，RoomViewModel + HallViewModel 共享 |

### 设计要点
- `KickCooldownStore` 为 Application 级单例（`AppContainer.kickCooldownStore`），RoomViewModel 写入、HallViewModel 读取，实现跨 ViewModel cooldown 拦截
- `Clock` 接口注入（SystemClock 默认实现），支持单元测试中注入 FakeClock
- `acknowledgeKick()` 保存 cooldown 并导航回大厅
- HallViewModel.enterRoom() 进入前检查 cooldown，未过期则拦截并提示

---

## T-30043 公告栏 + 管理员徽章 + RoomInfoUpdated

### 概述
进房自动弹出公告（24h 防重），顶部图标支持手动查看，角色徽章展示 Owner/Admin，WS 动态刷新。

### 组件结构
| 组件 | 职责 |
|------|------|
| `AnnouncementPopup` | AlertDialog，长文本支持 verticalScroll，dismissOnClickOutside=true |
| `AnnouncementIcon` | 顶部 📄 图标，公告非空时显示，点击触发弹窗 |
| `RoleBadge` | Owner 👑 / Admin 🛡️ / Member 无显示；所有用户身份渲染位置统一复用 |
| `AnnouncementSeenStore` | 记录各房间上次弹窗时间戳，Application 单例，24h 防重复弹出 |

### 设计要点
- `AnnouncementSeenStore` 为 Application 级单例（`AppContainer.announcementSeenStore`），进出房间 ViewModel 重建不丢记录
- `Clock` 接口注入（与 T-30042 共享 `SystemClock` / `FakeClock`），24h 判断可测
- `RoomInfoUpdated` → 更新 roomState.announcement/title/category；announcement 变化时重置 seen + 重新弹窗
- `AdminChanged` → 更新 roomState.adminUserId，触发所有 `RoleBadge` 重组

### testTag
`announcement_popup`、`btn_announcement_close`、`btn_show_announcement`、`role_badge_{userId}`

---

## T-30044 禁麦/禁言 UI 反馈 + 抱麦集成

### 概述
本地 UI 对禁麦/禁言状态的即时响应，以及 ForceTakeMic/ForceLeaveMic 信令的自动处理。

### 组件结构
| 组件 | 职责 |
|------|------|
| `SelfGovernanceState` | 保存 micMutedUntil/chatMutedUntil（毫秒时间戳），提供 isMicMuted(nowMs)/isChatMuted(nowMs) 查询 |
| `IMicPermissionChecker` | 麦克风权限检查接口；AlwaysGrantedMicPermissionChecker 默认实现；FakeMicPermissionChecker 测试用 |
| `ChatInput.kt`（已修改） | `enabled = !selfGovernanceState.isChatMuted()`，placeholder 显示禁言剩余时间 |
| `MicSlot.kt`（已修改） | 禁麦时"+"按钮 enabled=false，点击时 ShowToast |

### 设计要点
- `SelfGovernanceState.isMicMuted(nowMs)` 与 `MuteCountdownViewModel` 互补：前者控制 UI 置灰防操作，后者控制倒计时 Chip 显示
- `ForceTakeMic(forcedBy != null, isSelf)`：无麦克风权限时 requestMicPermission() → 拒绝则自动发 LeaveMic
- `ForceLeaveMic(forcedBy != null, isSelf)`：stopPublishing() + onMicSelf=false + ShowToast "你已被抱下麦"
- `Clock` 接口注入（复用 T-30042 已有的 SystemClock/FakeClock）

### MEDIUM 遗留
- 权限拒绝回调中 `wsClient.send()` 未包裹 `viewModelScope.launch`（线程安全隐患，待后续迭代修复）

---

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
