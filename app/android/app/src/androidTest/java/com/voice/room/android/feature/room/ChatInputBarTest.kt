package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — ChatInputBar (T-30015)
 *
 * CI-01: 空文本 → chat_send_button disabled
 * CI-02: 只有空白字符 → chat_send_button disabled
 * CI-03: 有效文本 → chat_send_button enabled
 * CI-04: 点击发送按钮 → onSendMessage 回调收到正确文本
 * CI-05: 键盘 IME Send 动作 → 触发 onSendMessage
 * CI-06: testTag 验证：chat_input_field 和 chat_send_button 均可定位
 * CI-07: isSending=true → chat_send_button disabled（即便有文本）
 */
@RunWith(AndroidJUnit4::class)
class ChatInputBarTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // CI-01: 空文本 → chat_send_button disabled
    // ─────────────────────────────────────────────

    @Test
    fun CI01_emptyText_sendButtonIsDisabled() {
        composeTestRule.setContent {
            ChatInputBar(
                inputText = "",
                onInputTextChange = {},
                onSendMessage = {},
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_send_button").assertIsNotEnabled()
    }

    // ─────────────────────────────────────────────
    // CI-02: 只有空白字符 → chat_send_button disabled
    // ─────────────────────────────────────────────

    @Test
    fun CI02_blankText_sendButtonIsDisabled() {
        composeTestRule.setContent {
            ChatInputBar(
                inputText = "   ",
                onInputTextChange = {},
                onSendMessage = {},
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_send_button").assertIsNotEnabled()
    }

    // ─────────────────────────────────────────────
    // CI-03: 有效文本 → chat_send_button enabled
    // ─────────────────────────────────────────────

    @Test
    fun CI03_validText_sendButtonIsEnabled() {
        composeTestRule.setContent {
            ChatInputBar(
                inputText = "Hello",
                onInputTextChange = {},
                onSendMessage = {},
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_send_button").assertIsEnabled()
    }

    // ─────────────────────────────────────────────
    // CI-04: 点击发送按钮 → onSendMessage 回调收到正确文本
    // ─────────────────────────────────────────────

    @Test
    fun CI04_clickSendButton_onSendMessageCalledWithCorrectText() {
        var capturedMessage: String? = null
        val testText = "你好，世界！"

        composeTestRule.setContent {
            ChatInputBar(
                inputText = testText,
                onInputTextChange = {},
                onSendMessage = { capturedMessage = it },
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_send_button").performClick()
        composeTestRule.waitForIdle()

        assertEquals(testText, capturedMessage)
    }

    // ─────────────────────────────────────────────
    // CI-05: 键盘 IME Send 动作 → 触发 onSendMessage
    // ─────────────────────────────────────────────

    @Test
    fun CI05_imeAction_triggersSendMessage() {
        var capturedMessage: String? = null
        val testText = "IME Send Test"

        composeTestRule.setContent {
            ChatInputBar(
                inputText = testText,
                onInputTextChange = {},
                onSendMessage = { capturedMessage = it },
            )
        }
        composeTestRule.waitForIdle()

        // Perform IME action (Send) on the text field
        composeTestRule.onNodeWithTag("chat_input_field").performImeAction()
        composeTestRule.waitForIdle()

        assertEquals(testText, capturedMessage)
    }

    // ─────────────────────────────────────────────
    // CI-06: testTag 验证：chat_input_field 和 chat_send_button 均可定位
    // ─────────────────────────────────────────────

    @Test
    fun CI06_testTagProtocol_bothTagsLocatable() {
        composeTestRule.setContent {
            ChatInputBar(
                inputText = "",
                onInputTextChange = {},
                onSendMessage = {},
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_input_field").assertIsDisplayed()
        composeTestRule.onNodeWithTag("chat_send_button").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // CI-07: isSending=true → chat_send_button disabled（即便有文本）
    // ─────────────────────────────────────────────

    @Test
    fun CI07_isSendingTrue_sendButtonIsDisabledEvenWithText() {
        var capturedMessage: String? = null

        composeTestRule.setContent {
            ChatInputBar(
                inputText = "有内容的消息",
                onInputTextChange = {},
                onSendMessage = { capturedMessage = it },
                isSending = true,
            )
        }
        composeTestRule.waitForIdle()

        // 按钮应 disabled
        composeTestRule.onNodeWithTag("chat_send_button").assertIsNotEnabled()

        // onSendMessage 不应被触发
        assertNull(capturedMessage)
    }

    // ─────────────────────────────────────────────
    // 额外边界测试：空白文本 + IME 动作不触发 onSendMessage
    // ─────────────────────────────────────────────

    @Test
    fun CI05b_blankText_imeAction_doesNotTriggerSend() {
        var capturedMessage: String? = null

        composeTestRule.setContent {
            ChatInputBar(
                inputText = "  ",
                onInputTextChange = {},
                onSendMessage = { capturedMessage = it },
            )
        }
        composeTestRule.waitForIdle()

        // IME action on blank text should NOT trigger onSendMessage
        composeTestRule.onNodeWithTag("chat_input_field").performImeAction()
        composeTestRule.waitForIdle()

        assertNull(capturedMessage)
    }

    // ─────────────────────────────────────────────
    // 额外测试：onInputTextChange 在用户输入时被回调
    // ─────────────────────────────────────────────

    @Test
    fun CI_onInputTextChange_calledOnUserInput() {
        val changedValues = mutableListOf<String>()

        composeTestRule.setContent {
            ChatInputBar(
                inputText = "",
                onInputTextChange = { changedValues.add(it) },
                onSendMessage = {},
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_input_field").performTextInput("Hi")
        composeTestRule.waitForIdle()

        // onInputTextChange 应至少被调用一次，且最终值含有输入的字符
        assert(changedValues.isNotEmpty()) { "onInputTextChange never called" }
        assert(changedValues.last().contains("Hi")) {
            "Expected last value to contain 'Hi', got: ${changedValues.last()}"
        }
    }
}
