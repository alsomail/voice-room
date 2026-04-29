package com.voice.room.android.presentation

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * MainActivity Smoke Test — Round 2 BUG-002 修复
 *
 * MainActivity 是纯 Compose（无 XML layout），启动时默认显示 SplashScreen。
 * 验证 MainActivity 启动后 SplashScreen 的 splash_screen testTag 可见。
 */
@RunWith(AndroidJUnit4::class)
class MainActivitySmokeTest {
    @get:Rule
    val composeTestRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun launch_shows_auth_bootstrap_title() {
        // MainActivity → AppNavGraph(startDestination="splash") → SplashScreen
        // 验证 SplashScreen 的 splash_screen testTag 可见
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("splash_screen").assertIsDisplayed()
    }
}
