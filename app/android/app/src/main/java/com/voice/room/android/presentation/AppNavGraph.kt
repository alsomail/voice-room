package com.voice.room.android.presentation

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.voice.room.android.common.AppContainer
import com.voice.room.android.feature.auth.LoginScreen
import com.voice.room.android.feature.splash.SplashScreen
import com.voice.room.android.feature.splash.SplashViewModel

/**
 * AppNavGraph — Compose Navigation 全局导航骨架
 *
 * 路由：
 * - "splash" → SplashScreen（启动页，startDestination）
 * - "login"  → LoginScreen（登录页）
 * - "main"   → 主页占位（后续 T-30020 替换为三 Tab 框架）
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

        // ── 主页占位（后续 T-30020 替换为三 Tab 框架）──
        composable("main") {
            MainPlaceholderScreen(appContainer = appContainer)
        }
    }
}

/**
 * 主页占位 Composable — 保留旧 MainViewModel 的展示逻辑
 *
 * 后续 T-30020 将替换为真正的三 Tab 框架 (Hall / Profile / Settings)。
 */
@Composable
private fun MainPlaceholderScreen(appContainer: AppContainer) {
    val mainViewModel: MainViewModel = viewModel(
        factory = MainViewModel.Factory(appContainer)
    )
    val state = mainViewModel.uiState

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = state.title,
            style = MaterialTheme.typography.headlineMedium,
            color = MaterialTheme.colorScheme.onBackground
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = state.description,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}
