# 模块 4: 中东黑金主题与 App 壳体 (MENA Theme & App Shell)

> 返回 [任务总索引](./index.md)

## Phase 0.5: 交互壳体与基础体验

> **说明**：Phase 0 的代码已全部完成，但 Android App 仍停留在 Auth Bootstrap 调试页面，缺少完整的用户交互壳。Phase 0.5 聚焦于让 App "能看能用"：中东黑金视觉主题、Splash 启动页、主页三Tab框架、个人中心，以及对已有页面的视觉升级。Web 端补充解封确认弹窗和活水房间监控。  
> **产品设计规范**: 详见 [doc/product/android_app_design.md](../product/android_app_design.md)


## 模块 4: 中东黑金主题与 App 壳体 (MENA Theme & App Shell)

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|------------|
| **T-30018** | Android | Theme | MenaTheme 中东黑金主题系统 [TDS](../tds/android/T-30018.md) | 无 | 封装 Material3 黑金主题（Colors/Typography/Shapes）+ RTL Provider + GoldButton / GoldOutlinedTextField / AvatarWithFrame 通用组件 | 1. `MenaTheme {}` 内自动黑金色系<br>2. GoldButton 金色渐变+白字+24dp圆角<br>3. RTL 自动生效 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30018.md](../design/android/T-30018.md) |
| **T-30019** | Android | Splash | Splash 启动页 [TDS](../tds/android/T-30019.md) | T-30018 | 品牌 Splash 页：Logo 缩放动画 → JWT 检测 → 自动导航到 MainScreen 或 LoginScreen | 1. Logo 缩放+淡入动画 800ms<br>2. 有效 JWT → MainScreen<br>3. 无效 JWT → LoginScreen<br>4. 返回键不可回退到 Splash | 4h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30019.md](../design/android/T-30019.md) |
| **T-30020** | Android | Navigation | MainScreen 底部三Tab框架 [TDS](../tds/android/T-30020.md) | T-30018, T-30019 | BottomNavigation 三Tab（房间/消息/我的），Tab切换保持状态 | 1. 默认显示房间Tab<br>2. 三Tab可切换，选中项金色<br>3. Tab切换保持各页面状态 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30020.md](../design/android/T-30020.md) |
| **T-30021** | Android | Auth | 登录页视觉升级 [TDS](../tds/android/T-30021.md) | T-30018 | 将现有 LoginScreen 从白色主题改造为黑金风格：渐变背景 + GoldOutlinedTextField + GoldButton。功能逻辑不变 | 1. 深色渐变背景<br>2. 所有输入框用 GoldOutlinedTextField<br>3. 按钮用 GoldButton<br>4. **现有功能测试不回归** | 3h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30021.md](../tds/android/T-30021.md) |
| **T-30022** | Android | Room | 大厅页视觉升级 [TDS](../tds/android/T-30022.md) | T-30018, T-30020 | 将 HallScreen 改造为黑金风格：深色RoomCard + OnlineCountBadge + 顶部栏 + 分类横滑(占位)。Paging3 逻辑不变 | 1. RoomCard 深色底+圆角16dp<br>2. OnlineCountBadge 绿点+数字<br>3. 创建房间 FAB 金色<br>4. **Paging3不回归** | 5h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30022.md](../design/android/T-30022.md) |
| **T-30023** | Android | Messages | 消息Tab占位页 [TDS](../tds/android/T-30023.md) | T-30018, T-30020 | 通用 `PlaceholderScreen` Composable（`core/ui/`）+ `MessagesPlaceholder` 委托，消息 Tab 展示"即将上线"占位页 | 1. 消息Tab显示占位页<br>2. PlaceholderScreen 可复用<br>3. 深色背景 | 2h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | |
| **T-30024** | Android | Profile | 个人中心页 [TDS](../tds/android/T-30024.md) | T-30018, T-30020, T-30004 | "我的"Tab 页面：头像(AvatarWithFrame)+昵称+ID+余额+设置入口+退出登录(二次确认) | 1. 显示用户头像/昵称/ID/余额<br>2. 复制ID到剪贴板<br>3. 退出登录二次确认→清JWT→LoginScreen<br>4. 网络异常用本地缓存 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30024.md](../design/android/T-30024.md) |
| **T-30025** | Android | Room | 房间页视觉升级 [TDS](../tds/android/T-30025.md) | T-30018 | 将 RoomScreen 改造为黑金风格：主麦突出(80dp金色光圈) + 副麦4列 + 弹幕金色昵称 + 深色背景。WS/上下麦逻辑不变 | 1. 主麦80dp+金色光圈<br>2. 副麦60dp四列<br>3. 空麦位虚线+"+"<br>4. 系统消息金黄色居中<br>5. **WS/上下麦不回归** | 6h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30025.md](../design/android/T-30025.md) |
| **T-30026** | Android | Room | 房间底部操作栏升级 [TDS](../tds/android/T-30026.md) | T-30018, T-30025 | 底部操作栏扩展：输入框 + 🎤麦克风开关 + 🎁礼物(灰禁) + ❤️表情(灰禁) + 🚪退出(二次确认) | 1. 4个功能按钮可见<br>2. 🎤不在麦上时禁用<br>3. 🎤在麦上时绿/红切换<br>4. 🎁❤️灰色禁用+Toast<br>5. 🚪二次确认退出 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending | [T-30026.md](../design/android/T-30026.md) |

---
