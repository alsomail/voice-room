package com.voice.room.android.presentation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.auth.LoginScreen
import com.voice.room.android.feature.gift.GiftPanelViewModel
import com.voice.room.android.feature.main.MainScreen
import com.voice.room.android.feature.ranking.RankingScreen
import com.voice.room.android.feature.ranking.RankingViewModel
import com.voice.room.android.feature.room.CreateRoomScreen
import com.voice.room.android.feature.room.CreateRoomViewModel
import com.voice.room.android.feature.room.RoomScreen
import com.voice.room.android.feature.room.RoomViewModelFactory
import com.voice.room.android.feature.room.RoomViewState
import com.voice.room.android.feature.room.governance.MuteCountdownViewModel
import com.voice.room.android.feature.splash.SplashScreen
import com.voice.room.android.feature.splash.SplashViewModel

/**
 * AppNavGraph — Compose Navigation 全局导航骨架
 *
 * 路由：
 * - "splash"         → SplashScreen（启动页，startDestination）
 * - "login"          → LoginScreen（登录页）
 * - "main"           → MainScreen（三 Tab 框架，T-30020）
 * - "ranking"        → RankingScreen（魅力/财富榜页，T-30033）
 * - "create_room"    → CreateRoomScreen（T-30036 + T-30037，R1 HIGH-02 修复）
 * - "room/{roomId}"  → RoomScreen（BUG-ROOM-NAV 修复，房间内页）
 *
 * 导航规则：
 * - Splash → Main/Login 使用 popUpTo("splash") { inclusive = true } 防止返回
 * - Main → Room 由 MainScreen.onNavigateToRoom 透传，在此处 navigate("room/{roomId}")
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
                factory = SplashViewModel.Factory(
                    tokenManager = appContainer.tokenManager,
                    consentRepository = appContainer.consentRepository,
                )
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
            val loginViewModel: com.voice.room.android.feature.auth.LoginViewModel = viewModel(
                factory = com.voice.room.android.feature.auth.LoginViewModel.Factory(
                    authRepository = appContainer.authRepository,
                    tokenManager = appContainer.tokenManager,
                    unauthorizedHandler = appContainer.unauthorizedHandler,
                    analyticsPort = appContainer.analyticsPort,
                )
            )
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate("main") {
                        popUpTo("login") { inclusive = true }
                    }
                },
                loginViewModel = loginViewModel,
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
                // BUG-ROOM-NAV 修复：注入真实导航回调，由 outer navController 执行
                onNavigateToRoom = { roomId ->
                    navController.navigate("room/$roomId")
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

        // ── 房间内页 (BUG-ROOM-NAV 修复) ──────────────
        composable("room/{roomId}") { backStackEntry ->
            val roomId = backStackEntry.arguments?.getString("roomId") ?: ""

            // ── ViewModel 装配 ──────────────────────────────
            val roomViewModel: com.voice.room.android.feature.room.RoomViewModel = viewModel(
                factory = RoomViewModelFactory(
                    wsClient               = appContainer.webSocketClient,
                    roomSnapshotRepository = appContainer.roomSnapshotRepository,
                    kickCooldownStore      = appContainer.kickCooldownStore,
                    announcementSeenStore  = appContainer.announcementSeenStore,
                    tokenManager           = appContainer.tokenManager,
                    wsUrl                  = appContainer.environment.wsUrl,
                )
            )
            val giftPanelViewModel: GiftPanelViewModel = viewModel(
                factory = GiftPanelViewModel.factory(
                    giftRepository = appContainer.giftRepository,
                    wsClient       = appContainer.webSocketClient,
                    roomId         = roomId,
                    analyticsPort  = appContainer.analyticsPort,
                )
            )
            val muteCountdownViewModel = viewModel<MuteCountdownViewModel>()

            // ── 进房 ────────────────────────────────────────
            LaunchedEffect(roomId) {
                roomViewModel.joinRoom(roomId)
            }

            // ── 状态收集 ─────────────────────────────────────
            val viewState by roomViewModel.uiState.collectAsState()
            val giftUiState by giftPanelViewModel.uiState.collectAsState()

            // ── UI 渲染 ──────────────────────────────────────
            when (val state = viewState) {
                is RoomViewState.Loading -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator(color = MenaColors.Primary)
                    }
                }
                is RoomViewState.Error -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = state.message,
                            color = MenaColors.Error,
                        )
                    }
                }
                is RoomViewState.Success -> {
                    RoomScreen(
                        uiState                  = state.uiState,
                        events                   = roomViewModel.events,
                        kickedState              = roomViewModel.kickedState.collectAsState().value,
                        onAcknowledgeKick        = { roomViewModel.acknowledgeKick() },
                        muteCountdownViewModel   = muteCountdownViewModel,
                        giftUiState              = giftUiState,
                        giftEvents               = giftPanelViewModel.events,
                        onBack                   = {
                            roomViewModel.leaveRoom()
                            navController.popBackStack()
                        },
                        onSendMessage            = { text -> roomViewModel.sendMessage(text) },
                        onMicPermissionGranted   = { slotIndex -> roomViewModel.onMicPermissionGranted(slotIndex) },
                        onMicSlotClick           = { slotIndex -> roomViewModel.onMicSlotClick(slotIndex) },
                        onMicToggle              = { roomViewModel.toggleMicMute() },
                        onLeaveRoom              = {
                            roomViewModel.leaveRoom()
                            navController.popBackStack()
                        },
                        onConfirmLeaveMic        = { slotIndex -> roomViewModel.confirmLeaveMic(slotIndex) },
                        onSelectGift             = { giftId -> giftPanelViewModel.selectGift(giftId) },
                        onSelectCount            = { count -> giftPanelViewModel.selectCount(count) },
                        onSelectGiftTab          = { tab -> giftPanelViewModel.selectTab(tab) },
                        onSelectRecipient        = { userId -> giftPanelViewModel.selectRecipient(userId) },
                        onSendGift               = { giftPanelViewModel.sendGift() },
                        onGiftRechargeClick      = { giftPanelViewModel.onRechargeClick() },
                        onGiftRetry              = { giftPanelViewModel.retryLoad() },
                        onGiftPanelDismiss       = { giftPanelViewModel.dismiss() },
                        onGoToWalletClick        = { giftPanelViewModel.onGoToWallet() },
                        onNavigateToWallet       = { navController.navigate("wallet") },
                        onNavigateToRanking      = { navController.navigate("ranking") },
                    )
                }
            }
        }
    }
}
