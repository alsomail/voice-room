# Android App 界面设计规范

> **版本**: v1.0  
> **更新日期**: 2026-04-20  
> **参考竞品**: Yalla (NASDAQ: YALA)、YoHo、Ahlan  
> **设计语言**: 中东黑金风 + Material3 + RTL 优先

---

## 1. 设计总原则

### 1.1 中东视觉风格
- **主色调**: 黑金色系（深色背景 `#1A1A2E` + 金色强调 `#D4AF37` / `#FFD700`）
- **辅助色**: 深紫 `#16213E`、暗蓝 `#0F3460`
- **文字色**: 白色 `#FFFFFF`（主文字）、`#B0B0B0`（次要文字）、金色（VIP/重要标识）
- **字体**: 英文 Google Sans / Roboto、阿拉伯语 Noto Sans Arabic
- **圆角**: 大卡片 16dp、按钮 24dp（胶囊型）、头像 50%（圆形）
- **阴影**: 最小化，依靠色差和边框区分层级

### 1.2 RTL 布局规范
- 所有页面使用 `CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Rtl)`
- `Row` / `LazyRow` 自动镜像，`start` → 阿语的右侧
- 图标方向性（返回箭头、进度条）需镜像处理
- 数字和英文文本保持 LTR（不镜像）

### 1.3 导航架构
```
SplashScreen → LoginScreen → MainScreen (3 Tab)
                                 ├── Tab 1: HallScreen (房间列表)
                                 ├── Tab 2: MessagesScreen (IM消息)
                                 └── Tab 3: ProfileScreen (我的)
                                        
MainScreen → RoomScreen (房间详情)
```

---

## 2. 页面规格

### 2.1 Splash 启动页

**竞品参考**: Yalla 品牌 splash 2-3秒 → 自动判断登录态

**设计要素**:
- 全屏深色背景 (`#1A1A2E`)
- 居中 App Logo（金色调，带光效动画）
- 底部 App 版本号（白色小字）
- 自动判断：有有效 JWT → 直接跳转 MainScreen；无 → 跳转 LoginScreen

**动画**: Logo 从 0.5 缩放到 1.0 + 淡入（800ms Ease-Out）

**自动化测试锚点**:
- `testTag("splash_screen")` — 整个 Splash 容器
- `testTag("splash_logo")` — Logo 图片
- `testTag("splash_version")` — 版本号文字

---

### 2.2 登录页 (已有 T-30001，需升级视觉)

**竞品参考**: Yalla 极简登录，手机号+验证码，沙特区号默认

**现有能力**: LoginScreen / LoginViewModel / +966 格式 / 60s 倒计时 / RTL — 均已完成

**视觉升级需求**:
- 背景从白色改为深色渐变 (`#1A1A2E` → `#16213E`)
- 顶部大尺寸 Logo 金色
- 输入框改为深色底 + 金色边框 + 圆角
- "获取验证码"按钮改为金色胶囊按钮
- "登录"按钮改为金色渐变胶囊按钮
- 底部协议文字淡灰色

**自动化测试锚点** (已有):
- `testTag("phone_input")` — 手机号输入框
- `testTag("code_input")` — 验证码输入框
- `testTag("send_code_button")` — 发送验证码按钮
- `testTag("login_button")` — 登录按钮

---

### 2.3 主页框架 (MainScreen — 三 Tab)

**竞品参考**: Yalla 底部三Tab（广场/消息/我的），YoHo 底部四Tab

**设计要素**:
- 底部导航栏（BottomNavigation），深色背景
- 三个 Tab：
  - 🏠 **广场/大厅** (الغرف / Rooms) — 房间列表
  - 💬 **消息** (الرسائل / Messages) — IM 消息列表（Phase 0.5 占位）
  - 👤 **我的** (حسابي / Me) — 个人中心

**BottomNavigation 设计**:
- 背景: `#1A1A2E`
- 选中项: 金色图标 + 金色文字
- 未选中项: 灰色图标 `#6C6C6C` + 灰色文字
- 高度: 56dp
- 图标尺寸: 24dp

**自动化测试锚点**:
- `testTag("main_screen")` — 主页容器
- `testTag("bottom_nav")` — 底部导航栏
- `testTag("tab_rooms")` — 房间 Tab
- `testTag("tab_messages")` — 消息 Tab
- `testTag("tab_profile")` — 我的 Tab

---

### 2.4 房间大厅 Tab (HallScreen — 已有 T-30005/T-30006，需升级)

**竞品参考**: Yalla 广场页 — 顶部分类横滑 + 房间卡片网格

**现有能力**: LazyVerticalGrid + Coil头像 + Paging3 分页 — 已完成

**视觉升级需求**:
- 顶部：App 名称（金色）+ 搜索图标 + 创建房间按钮（金色圆形"+"）
- 房间分类横滑条（热门/新开/关注/游戏）— Phase 0.5 只做"热门"
- 房间卡片升级为深色底 + 圆角 16dp + 左下角房主小头像 + 右下角在线人数（带绿点动画）
- 空状态占位图（金色线条插画 + "还没有房间，去创建一个吧"）

**RoomCard 组件设计**:
```
┌─────────────────────┐
│ [房间封面/渐变底色]   │  ← 160dp 高
│                     │
│  🎤 房间名称         │  ← 白色 bodyMedium，最多2行
│  👤 房主昵称  🟢 87  │  ← 灰色 labelSmall + 绿色在线人数
└─────────────────────┘
```

**自动化测试锚点**:
- `testTag("hall_screen")` — 大厅容器
- `testTag("create_room_fab")` — 创建房间按钮
- `testTag("room_card_{roomId}")` — 房间卡片（动态 roomId）
- `testTag("room_list")` — 房间列表 Grid

---

### 2.5 消息 Tab (MessagesScreen — 占位页)

**Phase 0.5 作用**: IM 功能的占位入口，显示"即将上线"

**设计要素**:
- 深色背景
- 居中图标 + "消息功能即将上线" 灰色文字
- 预留 Scaffold 结构（顶部 TopAppBar + 内容区）

**自动化测试锚点**:
- `testTag("messages_screen")` — 消息页容器
- `testTag("messages_placeholder")` — 占位内容

---

### 2.6 我的 Tab (ProfileScreen)

**竞品参考**: Yalla "我的" — 头像+昵称+ID+余额+等级+设置入口

**设计要素**:
- 顶部区域（深紫渐变背景）：
  - 大头像（80dp 圆形，金色边框 2dp）
  - 昵称（白色 titleLarge）
  - ID 号（灰色 bodySmall，可复制）
  - VIP 标识（Phase 1，Phase 0.5 隐藏）
- 资产区域（Card）：
  - 钻石余额 💎（点击跳转充值 — Phase 1 占位）
- 功能列表：
  - 编辑资料（头像/昵称 — Phase 1）
  - 设置（语言切换/关于/退出登录）
  - 关于我们

**自动化测试锚点**:
- `testTag("profile_screen")` — 我的页容器
- `testTag("profile_avatar")` — 头像
- `testTag("profile_nickname")` — 昵称
- `testTag("profile_id")` — 用户 ID
- `testTag("profile_balance")` — 余额区域
- `testTag("btn_edit_profile")` — 编辑资料
- `testTag("btn_settings")` — 设置
- `testTag("btn_logout")` — 退出登录

---

### 2.7 房间页 (RoomScreen — 已有 T-30009/T-30010，需升级)

**竞品参考**: Yalla 房间页 — 顶部信息栏 + 麦位网格 + 弹幕列表 + 底部操作栏

**现有能力**: RoomTopBar + MicSlotsGrid + ChatMessageList + ChatInputBar — 均已完成

**视觉升级需求**:

**顶部信息栏**:
- 深色半透明背景
- 左(RTL右)：返回按钮
- 中：房间名称 + 房间 ID（小字灰色）
- 右(RTL左)：在线人数（🟢 数字）+ 更多菜单（⋮）

**麦位网格（核心）**:
- 主麦（1号麦）居中顶部，金色光圈，尺寸更大 (80dp)
- 其余麦位按 4+4 或 3+3+2 排列，尺寸 60dp
- 麦位状态三态：
  - 空闲: 虚线圆圈 + "+" 图标（灰色）
  - 占位: 用户头像 + 昵称（底部白色小字）+ 音浪动画（绿色波纹）
  - 静音: 用户头像 + 红色禁麦图标覆盖

**弹幕/聊天区**:
- 半透明深色底（`#1A1A2E` 80% 不透明度）
- 系统消息（进入/离开）居中黄色
- 用户消息：昵称金色 + 内容白色
- 礼物消息：特殊样式带礼物图标（Phase 1）
- 自动滚动到最新消息

**底部操作栏**:
- 深色背景
- 左(RTL右)：文字输入框（点击弹出键盘）
- 中间按钮组：🎤 麦克风开关 / 🎁 礼物面板(Phase 1 灰显) / ❤️ 表情
- 右(RTL左)：退出房间按钮

**自动化测试锚点** (在已有基础上扩展):
- `testTag("room_screen")` — 房间页容器
- `testTag("room_top_bar")` — 顶部信息栏
- `testTag("btn_back")` — 返回按钮
- `testTag("room_name")` — 房间名称
- `testTag("online_count")` — 在线人数
- `testTag("mic_slots_grid")` — 麦位网格（已有）
- `testTag("mic_slot_{index}")` — 麦位（已有类似）
- `testTag("chat_message_list")` — 弹幕列表（已有）
- `testTag("chat_input_field")` — 输入框（已有）
- `testTag("chat_send_button")` — 发送按钮（已有）
- `testTag("btn_mic_toggle")` — 麦克风开关
- `testTag("btn_gift")` — 礼物按钮
- `testTag("btn_emoji")` — 表情按钮
- `testTag("btn_exit_room")` — 退出房间

---

## 3. 通用组件提取

以下 UI 元素必须封装为独立可复用组件：

| 组件名 | 说明 | 使用场景 |
|--------|------|----------|
| `MenaTheme` | 中东黑金 Material3 主题封装 | 全局 |
| `GoldButton` | 金色渐变胶囊按钮 | 登录、创建房间、确认操作 |
| `GoldOutlinedTextField` | 深色底+金色边框输入框 | 登录页、创建房间、聊天输入 |
| `AvatarWithFrame` | 圆形头像 + 可选金色/VIP边框 | 麦位、个人中心、房间卡片 |
| `MicSlotCard` | 麦位三态卡片（已有，需升级视觉） | 房间页 |
| `RoomCard` | 房间列表卡片（已有，需升级视觉） | 大厅 |
| `MenaBottomNavigation` | 中东风格底部导航 | 主页框架 |
| `OnlineCountBadge` | 绿点 + 在线人数标签 | 房间卡片、房间顶栏 |
| `PlaceholderScreen` | 通用占位页（图标+文字） | 消息Tab等待开发的模块 |

---

## 4. 颜色令牌 (Design Tokens)

```kotlin
// MenaColors.kt
object MenaColors {
    val Background = Color(0xFF1A1A2E)
    val Surface = Color(0xFF16213E)
    val SurfaceVariant = Color(0xFF0F3460)
    val Primary = Color(0xFFD4AF37)       // 金色
    val PrimaryBright = Color(0xFFFFD700)  // 亮金色
    val OnBackground = Color(0xFFFFFFFF)
    val OnBackgroundSecondary = Color(0xFFB0B0B0)
    val OnBackgroundTertiary = Color(0xFF6C6C6C)
    val Error = Color(0xFFE74C3C)
    val Success = Color(0xFF2ECC71)
    val SystemMessage = Color(0xFFF39C12) // 系统消息黄色
}
```

---

## 5. 导航流程状态机

```
App Launch
    │
    ├── JWT 有效 → MainScreen (默认 Tab: Rooms)
    │
    └── JWT 无效/过期 → LoginScreen
                            │
                            └── 登录成功 → MainScreen
                            
MainScreen
    │
    ├── 点击房间卡片 → RoomScreen(roomId)
    │                      │
    │                      └── 返回 → MainScreen (Rooms Tab)
    │
    ├── 点击创建房间 → CreateRoomBottomSheet
    │                      │
    │                      └── 创建成功 → RoomScreen(newRoomId)
    │
    └── 退出登录 → LoginScreen
```
