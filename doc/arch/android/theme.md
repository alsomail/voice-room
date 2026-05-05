<!-- 
[AI 读写指令]
1. 本文件描述 Android 端 core/theme 模块的架构详情。
2. 当 theme 相关能力发生变更时，必须同步更新本文件并通知 index.md。
-->

# Android 中东黑金主题系统 (core/theme)

> **Task**: T-30018  
> **状态**: ✅ 已完成  
> **最后更新**: 2026-04-20

---

## 一、模块定位

`core/theme` 是 Android 端的全局设计系统基座，封装 Material3 黑金主题（Colors / Typography / Shapes）并提供 RTL 自动检测与通用 UI 组件，供所有 Feature 模块统一消费。

---

## 二、代码存放路径

```
app/android/app/src/main/java/com/voice/room/android/core/theme/
├── MenaColors.kt              # 黑金色彩令牌
├── MenaTypography.kt          # 排版规范
├── MenaShapes.kt              # 圆角规范
├── MenaTheme.kt               # 主题入口 Composable
├── GoldButton.kt              # 金色渐变胶囊按钮
├── GoldOutlinedTextField.kt   # 深色底 + 金色边框输入框
└── AvatarWithFrame.kt         # Coil 圆形头像 + 可选金色光圈
```

---

## 三、架构设计

```
┌─────────────────────────────────────────────────────┐
│                  MenaTheme {}                        │
│  (主题入口 Composable, MaterialTheme wrapper)        │
│                                                     │
│  ┌─────────────┐ ┌───────────────┐ ┌────────────┐  │
│  │ MenaColors  │ │MenaTypography │ │ MenaShapes │  │
│  │ 11 色彩令牌 │ │ 5 级排版      │ │ 3 级圆角   │  │
│  └─────────────┘ └───────────────┘ └────────────┘  │
│                                                     │
│  RTL 自动检测: TextUtils.getLayoutDirectionFromLocale│
│  darkColorScheme 强制深色                            │
└─────────────────────────────────────────────────────┘
         │
         ▼ 消费方（通用组件）
┌─────────────┐  ┌──────────────────────┐  ┌────────────────┐
│ GoldButton  │  │ GoldOutlinedTextField│  │AvatarWithFrame │
│ 金色渐变    │  │ 深色底+金色边框      │  │ Coil圆形+光圈  │
│ 胶囊 24dp   │  │ 输入框 12dp 圆角     │  │ 可选金色光圈   │
│ role=Button │  │                      │  │                │
└─────────────┘  └──────────────────────┘  └────────────────┘
```

---

## 四、核心模块详情

### 4.1 MenaColors — 色彩令牌

| 令牌 | 用途 | 类型 |
|------|------|------|
| Background | 全局背景 | darkColorScheme.background |
| Surface | 卡片/面板表面 | darkColorScheme.surface |
| SurfaceVariant | 次级表面 | darkColorScheme.surfaceVariant |
| Primary | 主色（金色） | darkColorScheme.primary |
| OnPrimary | 主色上文字 | darkColorScheme.onPrimary |
| Secondary | 辅色 | darkColorScheme.secondary |
| OnSecondary | 辅色上文字 | darkColorScheme.onSecondary |
| Error | 错误色 | darkColorScheme.error |
| OnError | 错误色上文字 | darkColorScheme.onError |
| OnBackground | 背景上文字 | darkColorScheme.onBackground |
| OnSurface | 表面上文字 | darkColorScheme.onSurface |
| ChatBubble | #2A2A2A 聊天气泡背景色（T-30052） | val ChatBubble |

**共 11 个色彩令牌**，全部通过 `darkColorScheme()` 构建。

### 4.2 MenaTypography — 排版规范

| 级别 | 字号 | 字体 | 用途 |
|------|------|------|------|
| Level 1 | 22sp | Roboto | 页面标题 |
| Level 2 | 16sp | Roboto | 正文/按钮 |
| Level 3 | 14sp | Roboto | 副文本 |
| Level 4 | 12sp | Roboto | 辅助文字 |
| Level 5 | 11sp | Roboto | 极小标注 |

### 4.3 MenaShapes — 圆角规范

| 级别 | 圆角 | 适用场景 |
|------|------|----------|
| Large | 16dp | 大卡片、底部弹窗 |
| Medium | 24dp | 按钮（胶囊形） |
| Small | 12dp | 输入框、小组件 |

### 4.4 MenaTheme — 主题入口

- **类型**: `@Composable` 函数
- **功能**: 包裹 `MaterialTheme`，注入 MenaColors / MenaTypography / MenaShapes
- **RTL 支持**: `TextUtils.getLayoutDirectionFromLocale(Locale.getDefault())` 自动检测当前系统语言方向
- **强制深色**: 始终使用 `darkColorScheme`，无浅色模式切换

### 4.5 GoldButton — 金色渐变胶囊按钮

- 金色线性渐变背景
- 白色文字
- 24dp 胶囊圆角
- `clickable(enabled, role = Button)` 语义标注，无障碍正确
- `enabled = false` 时视觉灰化 + 不可点击

### 4.6 GoldOutlinedTextField — 金色边框输入框

- 深色底色背景
- 金色描边（focused 时加亮）
- 12dp 圆角
- 支持 label / placeholder / error 状态

### 4.7 AvatarWithFrame — 圆形头像组件

- Coil `AsyncImage` 加载网络图片
- `CircleShape` clip 圆形裁剪
- 可选金色光圈边框（`showFrame: Boolean = false`）
- 占位图 + 错误图兜底

---

## 五、测试覆盖

| 类型 | 数量 | 位置 |
|------|------|------|
| JVM 单元测试 | 7 | `src/test/` |
| Android Instrumented | 26 | `src/androidTest/` |
| **总计** | **33** | — |

所有 33 个测试用例通过。

---

## 六、依赖关系

### 被依赖方（下游消费者）
- T-30019 Splash 启动页
- T-30020 MainScreen 底部三Tab框架
- T-30021 登录页视觉升级
- T-30022 大厅页视觉升级
- T-30023 消息Tab占位页
- T-30024 个人中心页
- T-30025 房间页视觉升级
- T-30026 房间底部操作栏升级

### 外部依赖
- Material3 Compose (`androidx.compose.material3`)
- Coil Compose (`io.coil-kt:coil-compose`)

---

## 七、关联文档

- [TDS 技术设计文档](../../tds/android/T-30018.md)
- [UI 设计文档](../../design/android/T-30018.md)
- [Android 架构总索引](./index.md)
