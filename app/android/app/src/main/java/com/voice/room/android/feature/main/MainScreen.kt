package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import com.voice.room.android.feature.profile.ProfileScreen
import com.voice.room.android.feature.wallet.WalletScreen
import com.voice.room.android.feature.room.CreateRoomBottomSheet
import com.voice.room.android.feature.room.CreateRoomViewModel
import com.voice.room.android.feature.room.HallScreen
import com.voice.room.android.feature.room.RoomListViewModel

/**
 * MainScreen — 三 Tab 主页框架 (T-30020, T-30022 升级)
 *
 * 使用 Scaffold + 内部 NavHost + NavigationBar 构建底部导航：
 * - 房间大厅（main/rooms）→ 复用 HallScreen（Paging3）+ CreateRoomBottomSheet
 * - 消息（main/messages）→ MessagesPlaceholder（占位）
 * - 我的（main/profile）→ ProfilePlaceholder（占位）
 *
 * T-30022 升级:
 * - HallScreen 新增 onCreateRoom 回调 → 控制 CreateRoomBottomSheet 显隐
 * - 创建成功后暂无导航（TODO: 接入 RoomScreen 导航）
 *
 * T-30024 升级:
 * - 将 ProfilePlaceholder() 替换为 ProfileScreen（真实个人中心页）
 * - 新增 onLogout 参数，退出登录后由调用方执行导航到 LoginScreen
 *
 * @param appContainer 依赖容器，提供 roomRepository / userRepository 等服务
 * @param onLogout     退出登录后的导航回调（由 AppNavGraph 注入）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(
    appContainer: AppContainer,
    onLogout: () -> Unit = {},
    onNavigateToRanking: () -> Unit = {},
) {
    val navController = rememberNavController()
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    // T-30022: CreateRoomBottomSheet 显隐控制
    var showCreateRoom by remember { mutableStateOf(false) }

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
                val roomListViewModel: RoomListViewModel = viewModel(
                    factory = RoomListViewModel.Factory(appContainer.roomRepository)
                )
                val pagingItems = roomListViewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(
                    pagingItems = pagingItems,
                    onNavigateToRoom = { /* TODO: 接入 RoomScreen 导航 */ },
                    onCreateRoom = { showCreateRoom = true },
                    onNavigateToRanking = onNavigateToRanking,
                )
            }
            composable(MainTab.MESSAGES.route) {
                MessagesPlaceholder()
            }
            composable(MainTab.PROFILE.route) {
                ProfileScreen(
                    appContainer = appContainer,
                    onLogout = onLogout,
                    onNavigateToWallet = { navController.navigate("wallet") },
                )
            }
            // ── 钱包页（T-30027）─────────────────────────────
            composable("wallet") {
                WalletScreen(
                    appContainer = appContainer,
                    onNavigateBack = { navController.popBackStack() },
                    onNavigateToLogin = onLogout,
                )
            }
        }
    }

    // T-30022: CreateRoomBottomSheet
    if (showCreateRoom) {
        val createRoomViewModel: CreateRoomViewModel = viewModel(
            factory = CreateRoomViewModel.Factory
        )
        CreateRoomBottomSheet(
            onSuccess = { /* TODO: 导航到新创建的房间 */ },
            onDismiss = { showCreateRoom = false },
            viewModel = createRoomViewModel,
        )
    }
}
