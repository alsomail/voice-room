package com.voice.room.android.core.theme

import androidx.activity.ComponentActivity
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — GoldOutlinedTextField (T-30018)
 *
 * GT-01: 输入框可见 assertIsDisplayed()
 * GT-02: 输入文字后 onValueChange 回调触发且值正确
 * GT-03: label 文本正确显示
 * GT-04: placeholder 在空值时可见
 */
@RunWith(AndroidJUnit4::class)
class GoldOutlinedTextFieldTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // GT-01: 输入框可见
    // ─────────────────────────────────────────────

    @Test
    fun GT01_textField_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                GoldOutlinedTextField(
                    value = "",
                    onValueChange = {},
                    modifier = Modifier.testTag("gold_tf")
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("gold_tf").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // GT-02: 输入文字后 onValueChange 回调触发且值正确
    // ─────────────────────────────────────────────

    @Test
    fun GT02_inputText_triggersOnValueChange() {
        val changedValues = mutableListOf<String>()
        composeTestRule.setContent {
            MenaTheme {
                var text by mutableStateOf("")
                GoldOutlinedTextField(
                    value = text,
                    onValueChange = {
                        text = it
                        changedValues.add(it)
                    },
                    modifier = Modifier.testTag("gold_tf")
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("gold_tf").performTextInput("Hello")
        composeTestRule.waitForIdle()

        assertTrue("onValueChange should have been called", changedValues.isNotEmpty())
        assertTrue(
            "Last changed value should contain 'Hello', got: ${changedValues.lastOrNull()}",
            changedValues.last().contains("Hello")
        )
    }

    // ─────────────────────────────────────────────
    // GT-03: label 文本正确显示
    // ─────────────────────────────────────────────

    @Test
    fun GT03_label_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                GoldOutlinedTextField(
                    value = "",
                    onValueChange = {},
                    label = "Room Name",
                    modifier = Modifier.testTag("gold_tf")
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("Room Name").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // GT-04: placeholder 在空值时可见
    // ─────────────────────────────────────────────

    @Test
    fun GT04_placeholder_visibleWhenEmpty() {
        composeTestRule.setContent {
            MenaTheme {
                GoldOutlinedTextField(
                    value = "",
                    onValueChange = {},
                    placeholder = "Enter room name...",
                    modifier = Modifier.testTag("gold_tf")
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("Enter room name...").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // 额外：阿拉伯语文本输入
    // ─────────────────────────────────────────────

    @Test
    fun GT_arabicInput_works() {
        val changedValues = mutableListOf<String>()
        composeTestRule.setContent {
            MenaTheme {
                var text by mutableStateOf("")
                GoldOutlinedTextField(
                    value = text,
                    onValueChange = {
                        text = it
                        changedValues.add(it)
                    },
                    modifier = Modifier.testTag("gold_tf")
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("gold_tf").performTextInput("مرحبا")
        composeTestRule.waitForIdle()

        assertTrue("onValueChange should have been called for Arabic input", changedValues.isNotEmpty())
    }
}
