package com.voice.room.android.feature.main

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.im.NoOpIMService
import com.voice.room.android.core.config.InMemoryRemoteConfigService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.telemetry.NoOpAnalyticsService
import com.voice.room.android.core.telemetry.NoOpCrashReporter
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.auth.DebugAuthService
import com.voice.room.android.data.gift.DebugGiftRepository
import com.voice.room.android.data.room.DebugRoomGateway
import com.voice.room.android.data.room.DebugRoomSyncService
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.data.wallet.DebugWalletRepository
import com.voice.room.android.domain.local.ITokenManager
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * MainScreen Compose UI 集成测试 (T-30020)
 *
 * 验证底部三 Tab 框架的核心行为：
 * - TB-01: 默认启动显示房间 Tab 选中
 * - TB-02: 点击三个 Tab 均可正常切换
 * - TB-06: 房间 Tab 显示 HallScreen 内容
 * - TB-07: 消息 Tab 显示占位内容
 * - TB-08: 我的 Tab 显示占位内容
 * - TB-10: 底部导航栏始终可见
 * - TB-11: main_screen testTag 可定位
 */
@RunWith(AndroidJUnit4::class)
class MainScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    /** 构造测试用 AppContainer（全 Fake/NoOp 实现） */
    private fun createTestAppContainer(): AppContainer {
        val fakeTokenManager = object : ITokenManager {
            override suspend fun saveToken(token: String) {}
            override suspend fun getToken(): String? = "test-token"
            override suspend fun clearToken() {}
        }
        return AppContainer(
            environment = AppEnvironment(
                environmentName = "test",
                apiBaseUrl = "https://test.example.com/api",
                wsUrl = "wss://test.example.com/ws",
                analyticsEndpoint = "https://test.example.com/analytics"
            ),
            analyticsService = NoOpAnalyticsService(),
            crashReporter = NoOpCrashReporter(),
            remoteConfigService = InMemoryRemoteConfigService(),
            mediaService = NoOpMediaService(),
            imService = NoOpIMService(),
            authService = DebugAuthService(),
            roomGateway = DebugRoomGateway(),
            roomSyncService = DebugRoomSyncService(),
            walletRepository = DebugWalletRepository(),
            giftRepository = DebugGiftRepository(),
            roomRepository = FakeRoomRepository(),
            webSocketClient = FakeWebSocketClient(),
            tokenManager = fakeTokenManager,
        )
    }

    // ── TB-11: main_screen testTag 可在根容器上定位 ──────
    @Test
    fun TB11_mainScreen_testTag_isDisplayed() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("main_screen").assertIsDisplayed()
    }

    // ── TB-10: 底部导航栏始终可见 ──────────────────────
    @Test
    fun TB10_bottomNavigation_isDisplayed() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("bottom_nav").assertIsDisplayed()
    }

    // ── TB-01: 默认启动后，房间 Tab 为选中状态 ──────────
    @Test
    fun TB01_defaultRoute_isRoomsTab_selected() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("tab_rooms").assertIsDisplayed()
        composeTestRule.onNodeWithTag("tab_rooms").assertIsSelected()
        composeTestRule.onNodeWithTag("tab_messages").assertIsNotSelected()
        composeTestRule.onNodeWithTag("tab_profile").assertIsNotSelected()
    }

    // ── TB-02: 点击三个 Tab 均可正常切换 ────────────────
    @Test
    fun TB02_clickTabs_switchesContent() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        // 默认选中 Rooms
        composeTestRule.onNodeWithTag("tab_rooms").assertIsSelected()

        // 点击 Messages Tab
        composeTestRule.onNodeWithTag("tab_messages").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("tab_messages").assertIsSelected()
        composeTestRule.onNodeWithTag("tab_rooms").assertIsNotSelected()

        // 点击 Profile Tab
        composeTestRule.onNodeWithTag("tab_profile").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("tab_profile").assertIsSelected()
        composeTestRule.onNodeWithTag("tab_messages").assertIsNotSelected()

        // 点击回 Rooms Tab
        composeTestRule.onNodeWithTag("tab_rooms").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("tab_rooms").assertIsSelected()
    }

    // ── TB-06: 房间 Tab 显示 HallScreen 内容 ────────────
    @Test
    fun TB06_roomsTab_showsHallScreenContent() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        // FakeRoomRepository 默认返回 2 个房间，渲染后应能看到房间卡片或加载状态
        // HallScreen 在加载完成后会显示房间卡片或 hall_loading
        composeTestRule.onNodeWithTag("tab_rooms").assertIsSelected()
        // 等待 Paging3 加载
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule
                .onAllNodes(hasTestTag("room_card_id-1"))
                .fetchSemanticsNodes()
                .isNotEmpty() ||
            composeTestRule
                .onAllNodes(hasTestTag("hall_loading"))
                .fetchSemanticsNodes()
                .isNotEmpty()
        }
    }

    // ── TB-07: 消息 Tab 显示占位内容 ────────────────────
    @Test
    fun TB07_messagesTab_showsPlaceholder() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        // 点击 Messages Tab
        composeTestRule.onNodeWithTag("tab_messages").performClick()
        composeTestRule.waitForIdle()

        // 消息 Tab 应显示占位文本
        composeTestRule.onNodeWithText("Messages").assertIsDisplayed()
    }

    // ── TB-08: 我的 Tab 显示占位内容 ────────────────────
    @Test
    fun TB08_profileTab_showsPlaceholder() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        // 点击 Profile Tab
        composeTestRule.onNodeWithTag("tab_profile").performClick()
        composeTestRule.waitForIdle()

        // 我的 Tab 应显示占位文本
        composeTestRule.onNodeWithText("Me").assertIsDisplayed()
    }

    // ── TB-02 扩展: 三个 Tab 均有 testTag 且可见 ─────────
    @Test
    fun allThreeTabs_areDisplayed() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("tab_rooms").assertIsDisplayed()
        composeTestRule.onNodeWithTag("tab_messages").assertIsDisplayed()
        composeTestRule.onNodeWithTag("tab_profile").assertIsDisplayed()
    }
}
