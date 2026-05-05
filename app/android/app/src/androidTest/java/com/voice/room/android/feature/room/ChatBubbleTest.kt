package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.core.theme.MenaTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — UserMessageItem 气泡容器 (T-30052)
 *
 * CB-01: UserMessageItem 渲染 content 文字
 * CB-02: UserMessageItem 含 testTag("chat_bubble") 气泡容器节点
 * CB-03: UserMessageItem 渲染 senderNickname
 */
@RunWith(AndroidJUnit4::class)
class ChatBubbleTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun cb01_userMessageItem_renders_content() {
        composeTestRule.setContent {
            MenaTheme {
                UserMessageItem(
                    message = ChatMessageUi(
                        messageId = "1",
                        senderNickname = "Alice",
                        content = "hello",
                        timestamp = 0L,
                        messageType = MessageType.USER_TEXT,
                    ),
                )
            }
        }
        composeTestRule.onNodeWithText("hello").assertIsDisplayed()
    }

    @Test
    fun cb02_userMessageItem_has_bubble_container() {
        composeTestRule.setContent {
            MenaTheme {
                UserMessageItem(
                    message = ChatMessageUi(
                        messageId = "2",
                        senderNickname = "Bob",
                        content = "bubble test",
                        timestamp = 0L,
                        messageType = MessageType.USER_TEXT,
                    ),
                )
            }
        }
        composeTestRule.onNodeWithTag("chat_bubble").assertIsDisplayed()
    }

    @Test
    fun cb03_nickname_renders() {
        composeTestRule.setContent {
            MenaTheme {
                UserMessageItem(
                    message = ChatMessageUi(
                        messageId = "3",
                        senderNickname = "Charlie",
                        content = "content",
                        timestamp = 0L,
                        messageType = MessageType.USER_TEXT,
                    ),
                )
            }
        }
        composeTestRule.onNodeWithText("Charlie").assertIsDisplayed()
    }
}
