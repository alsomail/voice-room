package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.paging.compose.collectAsLazyPagingItems
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.room.HallScreen
import com.voice.room.android.feature.room.RoomListViewModel

/**
 * MainScreen — 三 Tab 主页框架 (T-30020)
 *
 * 使用 Scaffold + 内部 NavHost + NavigationBar 构建底部导航：
 * - 房间大厅（main/rooms）→ 复用 HallScreen（Paging3）
 * - 消息（main/messages）→ MessagesPlaceholder（占位）
 * - 我的（main/profile）→ ProfilePlaceholder（占位）
 *
 * 嵌套导航：MainScreen 拥有独立 NavController，与外层 AppNavGraph 隔离。
 * Tab 切换使用 saveState/restoreState/launchSingleTop 标准模式保持页面状态。
 *
 * @param appContainer 依赖容器，提供 roomRepository 等服务
 */
@Composable
fun MainScreen(appContainer: AppContainer) {
    val navController = rememberNavController()
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    Scaffold(
        modifier = Modifier.testTag("main_screen"),
        containerColor = MenaColors.Background,
        bottomBar = {
            MenaBottomNavigation(
                currentRoute = currentRoute,
                onTabSelected = { tab ->
                    navController.navigate(tab.route) {
                        popUpTo(navController.graph.findStartDestination().id) {
                            saveState = true
                        }
                        launchSingleTop = true
                        restoreState = true
                    }
                }
            )
        }
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = MainTab.ROOMS.route,
            modifier = Modifier.padding(innerPadding)
        ) {
            composable(MainTab.ROOMS.route) {
                // 复用现有 HallScreen (Paging3)
                // onRoomClick 暂传空回调，T-30022 视觉升级时完善导航
                val roomListViewModel: RoomListViewModel = viewModel(
                    factory = RoomListViewModel.Factory(appContainer.roomRepository)
                )
                val pagingItems = roomListViewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(
                    pagingItems = pagingItems,
                    onNavigateToRoom = { /* TODO: T-30022 接入 RoomScreen 导航 */ }
                )
            }
            composable(MainTab.MESSAGES.route) {
                MessagesPlaceholder()
            }
            composable(MainTab.PROFILE.route) {
                ProfilePlaceholder()
            }
        }
    }
}
