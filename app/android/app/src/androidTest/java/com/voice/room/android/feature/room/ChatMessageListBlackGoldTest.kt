package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — ChatMessageList 黑金色彩 (T-30025)
 *
 * VS-10: USER_TEXT 消息昵称使用金色（MenaColors.Primary #D4AF37），通过节点可见性验证
 * VS-11: SYSTEM_NOTICE 消息使用金黄色（MenaColors.SystemMessage #F39C12），通过节点可见性验证
 * VS-12: 系统消息居中对齐（TextAlign.Center）— 节点可见 + 内容正确
 *
 * 注意：Compose UI Test 框架无法直接断言 Text 的 color 值。
 * 颜色正确性通过源码审查 + Color 常量单测保证（MenaColorsTest）。
 * 此处测试节点的可见性和文字内容，确保颜色修改不引入崩溃或文本丢失。
 */
@RunWith(AndroidJUnit4::class)
class ChatMessageListBlackGoldTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ── VS-10: 用户消息金色昵称 ——————————————————————————————————————————————

    /**
     * VS-10: USER_TEXT 消息昵称文本可见，节点可通过 user_message_{index} 定位
     * 颜色从 MaterialTheme.colorScheme.primary 改为 MenaColors.Primary (#D4AF37)
     */
    @Test
    fun VS10_user_message_nickname_visible_with_mena_primary_color() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "u1",
                senderNickname = "أحمد",   // 阿拉伯语昵称，测试 Unicode
                content = "مرحبا",
                timestamp = 0L,
                messageType = MessageType.USER_TEXT,
            ),
        )
        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        // user_message_0 节点存在
        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        // 昵称文字可见
        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("أحمد")
        // 内容文字可见
        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("مرحبا")
    }

    /**
     * VS-10 扩展: 多用户消息昵称均可见（颜色修改不影响多节点渲染）
     */
    @Test
    fun VS10_ext_multiple_user_messages_all_nicknames_visible() {
        val messages = listOf(
            ChatMessageUi(messageId = "u1", senderNickname = "Alice", content = "Hello", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "u2", senderNickname = "Bob", content = "Hi", timestamp = 1L, messageType = MessageType.USER_TEXT),
        )
        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("Alice")
        composeTestRule.onNodeWithTag("user_message_1").assertTextContains("Bob")
    }

    // ── VS-11: 系统消息金黄色 ——————————————————————————————————————————————————

    /**
     * VS-11: SYSTEM_NOTICE 消息文本可见，使用 MenaColors.SystemMessage 颜色（#F39C12）
     */
    @Test
    fun VS11_system_notice_message_visible_with_mena_system_message_color() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "sys1",
                senderNickname = null,
                content = "مستخدم دخل الغرفة",   // "用户进入了房间"（阿拉伯语）
                timestamp = 0L,
                messageType = MessageType.SYSTEM_NOTICE,
            ),
        )
        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        // system_message_0 节点存在
        composeTestRule.onNodeWithTag("system_message_0").assertIsDisplayed()
        // 内容文字可见
        composeTestRule.onNodeWithTag("system_message_0").assertTextContains("مستخدم دخل الغرفة")
        // 用户消息节点不存在（正确区分消息类型）
        composeTestRule.onNodeWithTag("user_message_0").assertDoesNotExist()
    }

    // ── VS-12: 系统消息居中对齐 ——————————————————————————————————————————————

    /**
     * VS-12: 系统消息居中对齐（TextAlign.Center + Box contentAlignment=Center）
     * 通过节点可见 + 文本正确断言（排版布局正确性由源码保证）
     */
    @Test
    fun VS12_system_notice_rendered_centered_alignment() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "sys1",
                senderNickname = null,
                content = "Bob 进入了房间",
                timestamp = 0L,
                messageType = MessageType.SYSTEM_NOTICE,
            ),
        )
        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        // system_message_0 可见，文字内容正确
        composeTestRule.onNodeWithTag("system_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("system_message_0").assertTextContains("Bob 进入了房间")
    }

    // ── 混合消息颜色不回归 ——————————————————————————————————————————————————

    /**
     * 混合消息：USER_TEXT + SYSTEM_NOTICE 混合出现时均正常渲染（颜色修改不破坏混合场景）
     */
    @Test
    fun VS10_VS11_mixed_messages_both_rendered_correctly() {
        val messages = listOf(
            ChatMessageUi(messageId = "u1", senderNickname = "Alice", content = "大家好", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s1", senderNickname = null, content = "Bob 进入了房间", timestamp = 1L, messageType = MessageType.SYSTEM_NOTICE),
            ChatMessageUi(messageId = "u2", senderNickname = "Bob", content = "欢迎~", timestamp = 2L, messageType = MessageType.USER_TEXT),
        )
        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("system_message_1").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_2").assertIsDisplayed()

        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("Alice")
        composeTestRule.onNodeWithTag("system_message_1").assertTextContains("Bob 进入了房间")
    }
}
