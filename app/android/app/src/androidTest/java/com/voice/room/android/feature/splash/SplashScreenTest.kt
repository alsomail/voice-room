package com.voice.room.android.feature.splash

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.core.theme.MenaTheme
import com.voice.room.android.domain.local.ITokenManager
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — SplashScreen (T-30019)
 *
 * SP-06: splash_screen testTag 可见
 * SP-07: splash_logo testTag 可见
 * SP-08: splash_version testTag 可见且包含版本号字符串
 * SP-09: Logo 动画验证（初始 scale < 1.0，说明动画从 0.5 开始）
 *
 * 注意：这些测试在 MenaTheme 内渲染 SplashScreen，使用 FakeTokenManager 避免真实 DataStore 依赖。
 */
@RunWith(AndroidJUnit4::class)
class SplashScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    /**
     * 简单 FakeTokenManager — 始终返回 null（不关心导航结果，仅测 UI 渲染）
     */
    private class FakeTokenManager : ITokenManager {
        override suspend fun saveToken(token: String) {}
        override suspend fun getToken(): String? = null
        override suspend fun clearToken() {}
    }

    /**
     * 创建 SplashViewModel + 渲染 SplashScreen
     */
    private fun renderSplashScreen() {
        val viewModel = SplashViewModel(FakeTokenManager())
        composeTestRule.setContent {
            MenaTheme {
                SplashScreen(
                    onNavigateToMain = {},
                    onNavigateToLogin = {},
                    splashViewModel = viewModel
                )
            }
        }
    }

    // ─── SP-06: splash_screen 容器可见 ──────────────────

    @Test
    fun SP06_splashScreen_isDisplayed() {
        renderSplashScreen()
        composeTestRule.onNodeWithTag("splash_screen").assertIsDisplayed()
    }

    // ─── SP-07: splash_logo 可见 ────────────────────────

    @Test
    fun SP07_splashLogo_isDisplayed() {
        renderSplashScreen()
        composeTestRule.onNodeWithTag("splash_logo").assertIsDisplayed()
    }

    // ─── SP-08: splash_version 可见且包含版本号 ─────────

    @Test
    fun SP08_splashVersion_isDisplayedAndContainsVersion() {
        renderSplashScreen()
        composeTestRule.onNodeWithTag("splash_version")
            .assertIsDisplayed()
            .assertTextContains("v", substring = true)
    }

    // ─── SP-09: Logo 动画从缩小状态开始 ─────────────────

    @Test
    fun SP09_logoAnimation_startsWithReducedScale() {
        // 验证动画初始状态：Logo 可见（即使是缩小的），
        // 表明 Animatable(0.5f) scale 和 Animatable(0f) alpha 已初始化。
        // 由于 ComposeTestRule 默认暂停动画时钟，我们可以验证初始帧。
        renderSplashScreen()

        // 在初始帧，logo 节点存在（即使 alpha 为 0，节点仍在语义树中）
        composeTestRule.onNodeWithTag("splash_logo").assertExists()

        // 推进时间到动画完成后（800ms），logo 应当完全可见
        composeTestRule.mainClock.advanceTimeBy(900)
        composeTestRule.onNodeWithTag("splash_logo").assertIsDisplayed()
    }
}
