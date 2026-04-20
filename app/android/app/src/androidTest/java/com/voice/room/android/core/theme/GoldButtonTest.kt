package com.voice.room.android.core.theme

import androidx.activity.ComponentActivity
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — GoldButton (T-30018)
 *
 * GB-01: 按钮可见且 assertIsDisplayed()
 * GB-02: 点击回调触发（performClick() → callback invoked）
 * GB-03: enabled=false 时 assertIsNotEnabled() 且点击不触发回调
 * GB-04: 文字内容正确显示（assertTextEquals(expectedText)）
 * GB-05: 语义节点存在 Role.Button
 */
@RunWith(AndroidJUnit4::class)
class GoldButtonTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // GB-01: 按钮可见
    // ─────────────────────────────────────────────

    @Test
    fun GB01_goldButton_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "Join Room",
                    onClick = {},
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_btn").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // GB-02: 点击回调触发
    // ─────────────────────────────────────────────

    @Test
    fun GB02_click_triggersCallback() {
        var clicked = false
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "Click Me",
                    onClick = { clicked = true },
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_btn").performClick()
        composeTestRule.waitForIdle()
        assertTrue("onClick should have been called", clicked)
    }

    // ─────────────────────────────────────────────
    // GB-03: enabled=false → disabled + click ignored
    // ─────────────────────────────────────────────

    @Test
    fun GB03_disabled_notEnabledAndClickIgnored() {
        var clicked = false
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "Disabled",
                    onClick = { clicked = true },
                    enabled = false,
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_btn").assertIsNotEnabled()
        composeTestRule.onNodeWithTag("gold_btn").performClick()
        composeTestRule.waitForIdle()
        assertEquals("onClick should NOT be called when disabled", false, clicked)
    }

    // ─────────────────────────────────────────────
    // GB-04: 文字内容正确显示
    // ─────────────────────────────────────────────

    @Test
    fun GB04_textContent_isCorrect() {
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "ادخل الغرفة",
                    onClick = {},
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_btn").assertTextEquals("ادخل الغرفة")
    }

    // ─────────────────────────────────────────────
    // GB-05: 语义节点存在 Role.Button
    // ─────────────────────────────────────────────

    @Test
    fun GB05_semanticRole_isButton() {
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "Test",
                    onClick = {},
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()

        val hasButtonRole = SemanticsMatcher.expectValue(
            SemanticsProperties.Role, Role.Button
        )
        composeTestRule.onNodeWithTag("gold_btn")
            .assert(hasButtonRole)
    }

    // ─────────────────────────────────────────────
    // 额外：enabled=true 时按钮可用
    // ─────────────────────────────────────────────

    @Test
    fun GB_enabledTrue_buttonIsEnabled() {
        composeTestRule.setContent {
            MenaTheme {
                GoldButton(
                    text = "Enabled",
                    onClick = {},
                    enabled = true,
                    modifier = Modifier.testTag("gold_btn")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_btn").assertIsEnabled()
    }
}
