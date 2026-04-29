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
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.user.UserProfile
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * MainScreen Compose UI 集成测试 (T-30020, T-30024 升级)
 *
 * 验证底部三 Tab 框架的核心行为：
 * - TB-01: 默认启动显示房间 Tab 选中
 * - TB-02: 点击三个 Tab 均可正常切换
 * - TB-06: 房间 Tab 显示 HallScreen 内容
 * - TB-07: 消息 Tab 显示占位内容
 * - TB-08: 我的 Tab 显示 ProfileScreen 内容（T-30024 升级：不再是占位"Me"）
 * - TB-10: 底部导航栏始终可见
 * - TB-11: main_screen testTag 可定位
 */
@RunWith(AndroidJUnit4::class)
class MainScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    /** FakeUserRepository — 立即返回固定 UserProfile */
    private class FakeUserRepository : IUserRepository {
        val profile = UserProfile(
            id = "u001",
            phone = "+966512345678",
            nickname = "TestUser",
            avatar = null,
            coinBalance = 1000L,
            vipLevel = 0,
            createdAt = "2026-01-01T00:00:00Z",
        )
        override suspend fun getMe(): Result<UserProfile> = Result.success(profile)
    }

    /** 构造测试用 AppContainer — 使用 forUnitTest() 工厂，再 copy 覆盖 Fake 依赖 */
    private fun createTestAppContainer(): AppContainer {
        val fakeTokenManager = object : ITokenManager {
            override suspend fun saveToken(token: String) {}
            override suspend fun getToken(): String? = "test-token"
            override suspend fun clearToken() {}
        }
        return AppContainer.forUnitTest().copy(
            tokenManager = fakeTokenManager,
            userRepository = FakeUserRepository(),
            roomRepository = FakeRoomRepository(),
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

    // ── TB-07: 消息 Tab 显示占位内容 (T-30023 升级: 中文占位文本) ──
    @Test
    fun TB07_messagesTab_showsPlaceholder() {
        composeTestRule.setContent {
            MainScreen(appContainer = createTestAppContainer())
        }
        composeTestRule.waitForIdle()

        // 点击 Messages Tab
        composeTestRule.onNodeWithTag("tab_messages").performClick()
        composeTestRule.waitForIdle()

        // 消息 Tab 应显示占位 (Round 3 BUG-002：文本来自 R.string.messages_placeholder_title，
        // 设备 locale 决定语言；改用 testTag 'placeholder_title' 唯一定位)。
        composeTestRule.onNodeWithTag("placeholder_title").assertIsDisplayed()
    }

    // ── TB-08: 我的 Tab 显示 ProfileScreen 内容（T-30024：不再是占位"Me"）──
    @Test
    fun TB08_profileTab_showsProfileScreenContent() {
        composeTestRule.setContent {
            MainScreen(
                appContainer = createTestAppContainer(),
                onLogout = {},
            )
        }
        composeTestRule.waitForIdle()

        // 点击 Profile Tab
        composeTestRule.onNodeWithTag("tab_profile").performClick()
        composeTestRule.waitForIdle()

        // T-30024: ProfileScreen 替换了 ProfilePlaceholder
        // FakeUserRepository 立即返回成功，所以 profile_screen testTag 可见
        // 等待 Loading → Success 完成（最多 5s）
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule
                .onAllNodes(hasTestTag("profile_screen"))
                .fetchSemanticsNodes()
                .isNotEmpty()
        }
        composeTestRule.onNodeWithTag("profile_screen").assertIsDisplayed()
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
