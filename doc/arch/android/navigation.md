# Android 导航架构 (Navigation)

> **关联 Task**: T-30020 MainScreen 底部三Tab框架  
> **包路径**: `com.voice.room.android.feature.main`  
> **前置依赖**: T-30018 (MenaTheme), T-30019 (Splash + AppNavGraph)

---

## 一、AppNavGraph 全局路由骨架

`presentation/AppNavGraph.kt` 定义了应用的一级 Compose Navigation 路由：

```
startDestination = "splash"

splash  →  SplashScreen（品牌动画 + JWT 检测）
login   →  LoginScreen（手机号 + 验证码登录）
main    →  MainScreen（底部三Tab框架）
```

**导航策略**：
- `popUpTo("splash") { inclusive = true }` — Splash 导航后从回退栈中移除自身，禁止返回键回退到 Splash。
- `splash → login` 和 `splash → main` 均为单次导航，由 `SplashNavEvent` 一次性事件驱动。

---

## 二、MainScreen 三Tab框架

### 2.1 Tab 定义 — `MainTab` enum

| Tab | label | icon | 路由 key |
|-----|-------|------|----------|
| `Hall` | 大厅 | `Icons.Default.Home` | `"hall"` |
| `Messages` | 消息 | `Icons.Default.Email` | `"messages"` |
| `Profile` | 个人中心 | `Icons.Default.Person` | `"profile"` |

### 2.2 MainScreen 结构

```
Scaffold {
    content = NavHost(navController = tabNavController, startDestination = "hall") {
        composable("hall")     { HallScreen / RoomListScreen }
        composable("messages") { MessagesPlaceholder }
        composable("profile")  { ProfilePlaceholder }
    }
    bottomBar = MenaBottomNavigation(tabs, selectedTab, onTabSelected)
}
```

- **默认 Tab**：`Hall`（大厅），应用进入 MainScreen 后首屏显示房间列表。
- **Tab 切换状态保持**：使用 `saveState = true` / `restoreState = true` 策略，切换 Tab 时保持各页面的滚动位置和 ViewModel 状态，避免重新加载。
- **回退栈管理**：`popUpTo(tabNavController.graph.findStartDestination().id) { saveState = true }` + `launchSingleTop = true`，保证回退栈不堆积重复路由。

### 2.3 MenaBottomNavigation 金色主题底部导航

- 继承 Material3 `NavigationBar`，使用 `MenaTheme` 黑金色系：
  - **背景色**：`MenaColors.Surface`（深色）
  - **选中项**：`MenaColors.Primary`（金色）图标 + 文字
  - **未选中项**：`MenaColors.OnSurface`（灰色）图标 + 文字
- 每个 `NavigationBarItem` 配置 `icon` + `label` + `selected` 状态。
- RTL 布局自动适配（Compose `NavigationBar` 天然支持）。

---

## 三、Tab 内导航 saveState/restoreState 策略

```kotlin
navController.navigate(tab.route) {
    popUpTo(navController.graph.findStartDestination().id) {
        saveState = true
    }
    launchSingleTop = true
    restoreState = true
}
```

| 参数 | 作用 |
|------|------|
| `saveState = true` | 离开当前 Tab 时，将该 Tab 的回退栈状态（含 ViewModel、滚动位置）保存到 `NavBackStackEntry` |
| `restoreState = true` | 回到已访问过的 Tab 时，恢复之前保存的状态而非重建 |
| `launchSingleTop = true` | 防止重复点击同一 Tab 时创建多个相同目的地实例 |
| `popUpTo(startDest)` | 保证回退栈始终以 `Hall` 为根，避免深层回退栈堆积 |

---

## 四、文件清单

| 文件 | 说明 |
|------|------|
| `feature/main/MainScreen.kt` | Scaffold + NavHost + MenaBottomNavigation 组合 |
| `feature/main/MainTab.kt` | MainTab enum 定义三个 Tab 的 route/label/icon |
| `feature/main/MenaBottomNavigation.kt` | 金色主题底部导航栏 Composable |
| `presentation/AppNavGraph.kt` | 全局一级路由（splash/login/main），main 路由指向 MainScreen |
