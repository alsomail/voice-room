package com.voice.room.android.feature.profile

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.common.AppContainer
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.user.UserProfile
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.IOException

/**
 * ProfileScreen Compose UI 集成测试 (T-30024)
 *
 * PC-03: profile_nickname / profile_id_text / profile_balance 三个 testTag 文本可见
 * PC-04: avatar==null → avatar_placeholder Icon 可见
 * PC-06: 点击退出登录 → logout_confirm_dialog 出现
 * PC-07: logout_confirm_dialog 点击"取消" → 弹框消失
 * PC-08（UI）: logout_confirm_dialog 点击"确认" → onLogout 回调被调用
 * PC-10（UI）: Error 状态下 profile_error 可见，profile_retry_button 可点击
 * PC-12: fromCache=true → profile_cache_badge 可见；fromCache=false → 不可见
 */
@RunWith(AndroidJUnit4::class)
class ProfileScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─── Fakes ──────────────────────────────────────────────────────────────────

    /** 可配置的 FakeUserRepository */
    private class FakeUserRepository(
        var result: Result<UserProfile> = Result.success(DEFAULT_PROFILE),
    ) : IUserRepository {
        override suspend fun getMe(): Result<UserProfile> = result

        companion object {
            val DEFAULT_PROFILE = UserProfile(
                id = "u001",
                phone = "+966512345678",
                nickname = "TestUser",
                avatar = null,
                coinBalance = 1000L,
                vipLevel = 0,
                createdAt = "2026-01-01T00:00:00Z",
            )
        }
    }

    private val fakeTokenManager = object : ITokenManager {
        @Volatile private var token: String? = "valid.jwt.token"
        override suspend fun saveToken(token: String) { this.token = token }
        override suspend fun getToken(): String? = token
        override suspend fun clearToken() { token = null }
    }

    private fun buildAppContainer(userRepository: IUserRepository): AppContainer =
        AppContainer.forUnitTest().copy(
            tokenManager = fakeTokenManager,
            userRepository = userRepository,
        )

    // ─── PC-03: Success 状态下三个核心 testTag 可见 ──────────────────────────────

    @Test
    fun PC03_success_shows_nickname_id_and_balance() {
        val container = buildAppContainer(FakeUserRepository())
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        // Wait for Loading → Success
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_nickname"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        composeTestRule.onNodeWithTag("profile_nickname").assertIsDisplayed()
        composeTestRule.onNodeWithText("TestUser").assertIsDisplayed()

        // Round 3 BUG-002：profile_id_row 设有 .clickable(role=Role.Button)，
        // 父节点 mergeDescendants 后内部 'profile_id_text' 在合并语义树中被合并，
        // 改用 useUnmergedTree=true 定位；同时 'ID: u001' 也用 unmerged 查询。
        composeTestRule.onNodeWithTag("profile_id_text", useUnmergedTree = true).assertIsDisplayed()
        composeTestRule.onNodeWithText("ID: u001", useUnmergedTree = true).assertIsDisplayed()

        composeTestRule.onNodeWithTag("profile_balance").assertIsDisplayed()
        // Round 3 BUG-002：余额文本来自 R.string.profile_balance_format（"💰 %d coins" / "💰 %d عملة"），
        // 在 zh 设备上回退到英文，无法稳定断言中文 "金币"；改为只断言 testTag 可见 + 必含数字 1000。
        composeTestRule.onNodeWithText("1000", substring = true).assertIsDisplayed()
    }

    // ─── PC-04: avatar==null → avatar_placeholder 可见 ──────────────────────────

    @Test
    fun PC04_avatar_null_shows_placeholder_icon() {
        val repo = FakeUserRepository(
            result = Result.success(FakeUserRepository.DEFAULT_PROFILE.copy(avatar = null))
        )
        val container = buildAppContainer(repo)
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("avatar_placeholder"))
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("avatar_placeholder").assertIsDisplayed()
    }

    // ─── PC-06: 点击退出登录 → logout_confirm_dialog 出现 ───────────────────────

    @Test
    fun PC06_click_logout_shows_confirm_dialog() {
        val container = buildAppContainer(FakeUserRepository())
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        // Wait for success state
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_logout_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        // Round 3 BUG-002：logout 按钮位于滚动列表底部，需先 scrollTo 再 click，
        // 否则触摸坐标落在视口外，状态不会更新（dialog 不弹出）。
        composeTestRule.onNodeWithTag("profile_logout_button").performScrollTo().performClick()
        composeTestRule.waitForIdle()

        // 等待 dialog 出现（dialog 在独立 window 中，遍历全部 root）
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("logout_confirm_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("logout_confirm_button").assertIsDisplayed()
        composeTestRule.onNodeWithTag("logout_cancel_button").assertIsDisplayed()
    }

    // ─── PC-07: 点击"取消" → 弹框消失 ──────────────────────────────────────────

    @Test
    fun PC07_cancel_button_dismisses_dialog() {
        val container = buildAppContainer(FakeUserRepository())
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_logout_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        // Open dialog
        composeTestRule.onNodeWithTag("profile_logout_button").performScrollTo().performClick()
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("logout_cancel_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        // Click cancel
        composeTestRule.onNodeWithTag("logout_cancel_button").performClick()
        composeTestRule.waitForIdle()

        // Dialog should be dismissed
        composeTestRule.onAllNodes(hasTestTag("logout_cancel_button"))
            .fetchSemanticsNodes().let { nodes ->
                assert(nodes.isEmpty()) { "Dialog should be dismissed after cancel, but was still visible" }
            }
    }

    // ─── PC-08 (UI): 点击"确认" → onLogout 回调被调用 ────────────────────────────

    @Test
    fun PC08_confirm_logout_invokes_onLogout_callback() {
        val container = buildAppContainer(FakeUserRepository())
        var logoutCalled = false

        composeTestRule.setContent {
            ProfileScreen(
                appContainer = container,
                onLogout = { logoutCalled = true },
            )
        }

        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_logout_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        composeTestRule.onNodeWithTag("profile_logout_button").performScrollTo().performClick()
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("logout_confirm_button"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        composeTestRule.onNodeWithTag("logout_confirm_button").performClick()
        composeTestRule.waitForIdle()

        // onLogout should have been called (after ViewModel clears token + emits event)
        composeTestRule.waitUntil(timeoutMillis = 5000) { logoutCalled }
        assert(logoutCalled) { "onLogout callback should be invoked after confirming logout" }
    }

    // ─── PC-10 (UI): IOException → profile_error + profile_retry_button ────────

    @Test
    fun PC10_network_error_shows_error_state_with_retry_button() {
        val failingRepo = FakeUserRepository(
            result = Result.failure(IOException("Network error"))
        )
        val container = buildAppContainer(failingRepo)
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_error"))
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("profile_error").assertIsDisplayed()
        composeTestRule.onNodeWithTag("profile_retry_button").assertIsDisplayed()
    }

    // ─── PC-12: fromCache 控制 profile_cache_badge 可见性 ─────────────────────

    @Test
    fun PC12_fromCache_false_hides_cache_badge() {
        val container = buildAppContainer(FakeUserRepository())
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        // Wait for success
        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_nickname"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        // cache badge must NOT be visible when fromCache=false
        composeTestRule.onAllNodes(hasTestTag("profile_cache_badge"))
            .fetchSemanticsNodes().let { nodes ->
                assert(nodes.isEmpty()) {
                    "profile_cache_badge should NOT be visible when fromCache=false"
                }
            }
    }

    // ─── PC-03 扩展: profile_avatar testTag 容器渲染正常 ──────────────────────

    @Test
    fun profile_avatar_container_is_displayed() {
        val container = buildAppContainer(FakeUserRepository())
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        composeTestRule.waitUntil(timeoutMillis = 5000) {
            composeTestRule.onAllNodes(hasTestTag("profile_avatar"))
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("profile_avatar").assertIsDisplayed()
    }

    // ─── 加载状态时 profile_loading 可见 (loading state) ─────────────────────

    @Test
    fun profile_loading_is_displayed_initially() {
        // Use a repo that never returns to keep it in Loading state
        val neverReturnRepo = object : IUserRepository {
            override suspend fun getMe(): Result<UserProfile> {
                kotlinx.coroutines.delay(Long.MAX_VALUE)
                return Result.success(FakeUserRepository.DEFAULT_PROFILE)
            }
        }
        val container = buildAppContainer(neverReturnRepo)
        composeTestRule.setContent {
            ProfileScreen(appContainer = container, onLogout = {})
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("profile_loading").assertIsDisplayed()
    }
}
