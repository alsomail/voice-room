package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.CompositionLocalProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — ChatMessageList (T-30014)
 *
 * CL-01: 空消息列表 → 无崩溃，chat_message_list 可见
 * CL-02: USER_TEXT 消息 → user_message_0 可见，昵称和内容均显示
 * CL-03: SYSTEM_NOTICE 消息 → system_message_0 可见，内容文字可见
 * CL-04: 混合多条消息 → 所有 user_message_* / system_message_* 均可见
 * CL-05: testTag 协议验证：chat_message_list / user_message_0 / system_message_1 均可定位
 * CL-06: 初始 20 条消息 → 末尾 user_message_19 可见（自动滚动到底部）
 * CL-07: RTL 布局 → user_message_0 内容仍可见，无崩溃
 * CL-08: 含重复 messageId 的列表 → 只渲染去重后的条数，无崩溃
 */
@RunWith(AndroidJUnit4::class)
class ChatMessageListTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // CL-01: 空消息列表 → 无崩溃，chat_message_list 可见
    // ─────────────────────────────────────────────

    @Test
    fun CL01_emptyMessageList_nocrashAndListVisible() {
        composeTestRule.setContent {
            ChatMessageList(messages = emptyList())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_message_list").assertIsDisplayed()
        // No items rendered
        composeTestRule.onNodeWithTag("user_message_0").assertDoesNotExist()
        composeTestRule.onNodeWithTag("system_message_0").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // CL-02: USER_TEXT 消息 → user_message_0 可见，昵称和内容均显示
    // ─────────────────────────────────────────────

    @Test
    fun CL02_userTextMessage_showsNicknameAndContent() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "msg1",
                senderNickname = "Alice",
                content = "Hello World",
                timestamp = 0L,
                messageType = MessageType.USER_TEXT,
            ),
        )

        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("Alice")
        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("Hello World")
    }

    // ─────────────────────────────────────────────
    // CL-03: SYSTEM_NOTICE 消息 → system_message_0 可见，内容文字可见
    // ─────────────────────────────────────────────

    @Test
    fun CL03_systemNoticeMessage_showsCenteredContent() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "sys1",
                senderNickname = null,
                content = "Alice 进入了房间",
                timestamp = 0L,
                messageType = MessageType.SYSTEM_NOTICE,
            ),
        )

        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("system_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("system_message_0").assertTextContains("Alice 进入了房间")
        // USER_TEXT tag should not exist for this item
        composeTestRule.onNodeWithTag("user_message_0").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // CL-04: 混合多条消息 → 所有 user_message_* / system_message_* 均可见
    // ─────────────────────────────────────────────

    @Test
    fun CL04_mixedMessages_allRendered() {
        val messages = listOf(
            ChatMessageUi(messageId = "m1", senderNickname = "Alice", content = "Hi", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s1", senderNickname = null, content = "Bob 加入房间", timestamp = 1L, messageType = MessageType.SYSTEM_NOTICE),
            ChatMessageUi(messageId = "m2", senderNickname = "Bob", content = "Hey", timestamp = 2L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s2", senderNickname = null, content = "Carol 加入房间", timestamp = 3L, messageType = MessageType.SYSTEM_NOTICE),
            ChatMessageUi(messageId = "m3", senderNickname = "Carol", content = "Hello!", timestamp = 4L, messageType = MessageType.USER_TEXT),
        )

        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        // index 0: USER_TEXT
        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        // index 1: SYSTEM_NOTICE
        composeTestRule.onNodeWithTag("system_message_1").assertIsDisplayed()
        // index 2: USER_TEXT
        composeTestRule.onNodeWithTag("user_message_2").assertIsDisplayed()
        // index 3: SYSTEM_NOTICE
        composeTestRule.onNodeWithTag("system_message_3").assertIsDisplayed()
        // index 4: USER_TEXT
        composeTestRule.onNodeWithTag("user_message_4").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // CL-05: testTag 协议验证
    // ─────────────────────────────────────────────

    @Test
    fun CL05_testTagProtocol_allTagsLocatable() {
        val messages = listOf(
            ChatMessageUi(messageId = "u1", senderNickname = "User", content = "content", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s1", senderNickname = null, content = "系统消息", timestamp = 1L, messageType = MessageType.SYSTEM_NOTICE),
        )

        composeTestRule.setContent {
            ChatMessageList(messages = messages)
        }
        composeTestRule.waitForIdle()

        // 三个核心 testTag 必须均可定位
        composeTestRule.onNodeWithTag("chat_message_list").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("system_message_1").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // CL-06: 初始 20 条消息 → 末尾 user_message_19 可见（自动滚动底部）
    // ─────────────────────────────────────────────

    @Test
    fun CL06_twentyMessages_autoScrollsToBottom() {
        val messages = List(20) { i ->
            ChatMessageUi(
                messageId = "msg$i",
                senderNickname = "User$i",
                content = "Message $i",
                timestamp = i.toLong(),
                messageType = MessageType.USER_TEXT,
            )
        }

        composeTestRule.setContent {
            ChatMessageList(
                messages = messages,
                modifier = Modifier.height(300.dp),
            )
        }
        composeTestRule.waitForIdle()
        // 允许 animateScrollToItem 动画帧执行完毕
        composeTestRule.mainClock.advanceTimeBy(1_000)
        composeTestRule.waitForIdle()

        // 最后一条消息应当可见（已自动滚到底部）
        composeTestRule.onNodeWithTag("user_message_19").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // CL-07: RTL 布局 → user_message_0 内容仍可见，无崩溃
    // ─────────────────────────────────────────────

    @Test
    fun CL07_rtlLayout_userMessageDisplayedCorrectly() {
        val messages = listOf(
            ChatMessageUi(
                messageId = "rtl1",
                senderNickname = "مستخدم",
                content = "مرحبا بالجميع",
                timestamp = 0L,
                messageType = MessageType.USER_TEXT,
            ),
        )

        composeTestRule.setContent {
            CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Rtl) {
                ChatMessageList(messages = messages)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_0").assertTextContains("مرحبا بالجميع")
    }

    // ─────────────────────────────────────────────
    // CL-08: 含重复 messageId 的列表 → 只渲染去重后的条数，无崩溃
    // ─────────────────────────────────────────────

    @Test
    fun CL08_duplicateMessageIds_deduplicatedAndNocrash() {
        // 3 条消息，messageId "dup" 出现 2 次 → 去重后应只有 2 条
        val messagesWithDup = listOf(
            ChatMessageUi(messageId = "unique1", senderNickname = "Alice", content = "First", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "dup", senderNickname = "Bob", content = "Original", timestamp = 1L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "dup", senderNickname = "Bob", content = "Duplicate", timestamp = 2L, messageType = MessageType.USER_TEXT),
        )

        composeTestRule.setContent {
            ChatMessageList(messages = messagesWithDup)
        }
        composeTestRule.waitForIdle()

        // 去重后只有 2 条：unique1(index=0) 和 dup(index=1)
        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_1").assertIsDisplayed()
        // index=2 不存在（已去重）
        composeTestRule.onNodeWithTag("user_message_2").assertDoesNotExist()

        // "Duplicate" 内容不应出现（被去重掉）
        composeTestRule.onNodeWithTag("user_message_1").assertTextContains("Original")
    }
}
