package com.voice.room.android.core.theme

import androidx.activity.ComponentActivity
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertWidthIsEqualTo
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — AvatarWithFrame (T-30018)
 *
 * AF-01: 组件可见 assertIsDisplayed()
 * AF-02: showFrame=true 时金色边框存在（通过 testTag 断言）
 * AF-03: showFrame=false 时无边框（对应 testTag 不存在）
 * AF-04: imageUrl=null 时默认占位图显示
 * AF-05: 自定义 size=80.dp 时组件宽高 == 80dp（+ 边框宽度）
 */
@RunWith(AndroidJUnit4::class)
class AvatarWithFrameTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // AF-01: 组件可见
    // ─────────────────────────────────────────────

    @Test
    fun AF01_avatar_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("avatar").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // AF-02: showFrame=true → 金色边框 testTag 存在
    // ─────────────────────────────────────────────

    @Test
    fun AF02_showFrameTrue_frameTagExists() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    showFrame = true,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("avatar_frame").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // AF-03: showFrame=false → 边框 testTag 不存在
    // ─────────────────────────────────────────────

    @Test
    fun AF03_showFrameFalse_frameTagDoesNotExist() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    showFrame = false,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("avatar_frame").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // AF-04: imageUrl=null → 默认占位图显示
    // ─────────────────────────────────────────────

    @Test
    fun AF04_nullImageUrl_showsPlaceholder() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("avatar_placeholder").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // AF-05: size=80.dp → 组件宽高 == 84.dp (80 + 2*2dp border)
    // ─────────────────────────────────────────────

    @Test
    fun AF05_customSize_80dp_hasCorrectDimensions() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    size = 80.dp,
                    showFrame = true,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()

        // Total size = image size (80dp) + border (2dp * 2) = 84dp
        composeTestRule.onNodeWithTag("avatar")
            .assertWidthIsEqualTo(84.dp)
        composeTestRule.onNodeWithTag("avatar")
            .assertHeightIsEqualTo(84.dp)
    }

    // ─────────────────────────────────────────────
    // 额外：默认 size=60.dp → 宽高 == 64.dp (带边框)
    // ─────────────────────────────────────────────

    @Test
    fun AF_defaultSize_isCorrect() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    showFrame = true,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()

        // Default size = 60dp + border (2dp * 2) = 64dp
        composeTestRule.onNodeWithTag("avatar")
            .assertWidthIsEqualTo(64.dp)
        composeTestRule.onNodeWithTag("avatar")
            .assertHeightIsEqualTo(64.dp)
    }

    // ─────────────────────────────────────────────
    // 额外：showFrame=false 时无边框，宽高 == size
    // ─────────────────────────────────────────────

    @Test
    fun AF_noFrame_sizeEqualsExact() {
        composeTestRule.setContent {
            MenaTheme {
                AvatarWithFrame(
                    imageUrl = null,
                    size = 80.dp,
                    showFrame = false,
                    modifier = Modifier.testTag("avatar")
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("avatar")
            .assertWidthIsEqualTo(80.dp)
        composeTestRule.onNodeWithTag("avatar")
            .assertHeightIsEqualTo(80.dp)
    }
}
