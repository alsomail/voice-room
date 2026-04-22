package com.voice.room.android.presentation

import androidx.compose.runtime.Composable
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.voice.room.android.common.AppContainer
import com.voice.room.android.feature.auth.LoginScreen
import com.voice.room.android.feature.main.MainScreen
import com.voice.room.android.feature.ranking.RankingScreen
import com.voice.room.android.feature.ranking.RankingViewModel
import com.voice.room.android.feature.room.CreateRoomScreen
import com.voice.room.android.feature.room.CreateRoomViewModel
import com.voice.room.android.feature.splash.SplashScreen
import com.voice.room.android.feature.splash.SplashViewModel

/**
 * AppNavGraph — Compose Navigation 全局导航骨架
 *
 * 路由：
 * - "splash"      → SplashScreen（启动页，startDestination）
 * - "login"       → LoginScreen（登录页）
 * - "main"        → MainScreen（三 Tab 框架，T-30020）
 * - "ranking"     → RankingScreen（魅力/财富榜页，T-30033）
 * - "create_room" → CreateRoomScreen（T-30036 + T-30037，R1 HIGH-02 修复）
 *
 * 导航规则：
 * - Splash → Main/Login 使用 popUpTo("splash") { inclusive = true } 防止返回
 */
@Composable
fun AppNavGraph(appContainer: AppContainer) {
    val navController = rememberNavController()

    NavHost(
        navController = navController,
        startDestination = "splash"
    ) {
        // ── Splash 启动页 ──────────────────────────────
        composable("splash") {
            val splashViewModel: SplashViewModel = viewModel(
                factory = SplashViewModel.Factory(appContainer.tokenManager)
            )
            SplashScreen(
                splashViewModel = splashViewModel,
                onNavigateToMain = {
                    navController.navigate("main") {
                        popUpTo("splash") { inclusive = true }
                    }
                },
                onNavigateToLogin = {
                    navController.navigate("login") {
                        popUpTo("splash") { inclusive = true }
                    }
                }
            )
        }

        // ── 登录页 ────────────────────────────────────
        composable("login") {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate("main") {
                        popUpTo("login") { inclusive = true }
                    }
                }
            )
        }

        // ── 主页三 Tab 框架 (T-30020, T-30024 升级) ──────────────────
        composable("main") {
            MainScreen(
                appContainer = appContainer,
                onLogout = {
                    navController.navigate("login") {
                        popUpTo("main") { inclusive = true }
                    }
                },
                onNavigateToRanking = {
                    navController.navigate("ranking")
                },
            )
        }

        // ── 榜单页 (T-30033) ──────────────────────────
        composable("ranking") {
            val rankingViewModel: RankingViewModel = viewModel(
                factory = RankingViewModel.factory(appContainer.rankingRepository)
            )
            RankingScreen(
                viewModel = rankingViewModel,
                onNavigateBack = { navController.popBackStack() },
            )
        }

        // ── 创建房间页 (T-30036 + T-30037 R1 HIGH-02 修复) ────────────
        // CoverPickerBottomSheet 已内置于 CreateRoomScreen，无需此处注入
        composable("create_room") {
            val createRoomViewModel: CreateRoomViewModel = viewModel(
                factory = CreateRoomViewModel.Factory
            )
            CreateRoomScreen(
                viewModel = createRoomViewModel,
                onNavigateUp = { navController.popBackStack() },
                onNavigateToRoom = { roomId ->
                    navController.navigate("room/$roomId") {
                        popUpTo("create_room") { inclusive = true }
                    }
                },
                // onSelectCover 不传 → CreateRoomScreen 内置 CoverPickerBottomSheet 自动启用
            )
        }
    }
}
