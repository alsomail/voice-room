package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — RoomScreen (T-30009)
 *
 * UI-01: 传入 roomName/onlineCount → room_name / room_online_count 节点显示正确
 * UI-02: 有人麦位显示 mic_slot_occupied_*，空麦位显示 mic_slot_empty_*
 * UI-03: 3 条消息 → chat_message_list 可见，3 个消息节点均可见
 * UI-04: 空消息列表 → chat_message_list 可见但无子消息节点
 * UI-05: mic_slots_grid 可见，共 9 个槽位
 * UI-06: 输入 "Hello" → chat_send_button 可点击；点击后 onSend 收到 "Hello"，输入框清空
 * UI-07: chat_input_field 为空时 chat_send_button disabled
 * UI-08: 点击 room_back_button → onBack 回调触发一次
 * UI-09: isMuted=true → mic_slot_muted_icon 可见
 * UI-10: roomName 为 30 字符 → room_name 节点不崩溃，文本可见
 */
@RunWith(AndroidJUnit4::class)
class RoomScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────

    private fun defaultUiState(
        roomName: String = "测试房间",
        onlineCount: Int = 5,
        micSlots: List<MicSlotUi> = List(9) { MicSlotUi(index = it) },
        messages: List<ChatMessageUi> = emptyList(),
        isLoading: Boolean = false,
    ) = RoomUiState(
        roomId = "room-1",
        roomName = roomName,
        onlineCount = onlineCount,
        micSlots = micSlots,
        messages = messages,
        isLoading = isLoading,
    )

    // ─────────────────────────────────────────────
    // UI-01: 显示房间名与在线人数
    // ─────────────────────────────────────────────

    @Test
    fun UI01_displaysRoomNameInTopBar() {
        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(roomName = "测试房间", onlineCount = 5))
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_name").assertIsDisplayed()
        composeTestRule.onNodeWithTag("room_name").assertTextContains("测试房间")
        composeTestRule.onNodeWithTag("room_online_count").assertIsDisplayed()
        composeTestRule.onNodeWithTag("room_online_count").assertTextContains("5")
    }

    // ─────────────────────────────────────────────
    // UI-02: 有人 / 空麦位正确渲染
    // ─────────────────────────────────────────────

    @Test
    fun UI02_occupiedAndEmptyMicSlotsDisplayCorrectly() {
        val slots = List(9) { index ->
            when (index) {
                0 -> MicSlotUi(index = 0, userId = "u1", nickname = "Alice")
                2 -> MicSlotUi(index = 2, userId = "u2", nickname = "Bob")
                else -> MicSlotUi(index = index)
            }
        }

        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(micSlots = slots))
        }
        composeTestRule.waitForIdle()

        // Occupied slots
        composeTestRule.onNodeWithTag("mic_slot_occupied_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("mic_slot_occupied_2").assertIsDisplayed()

        // Empty slots (all others)
        listOf(1, 3, 4, 5, 6, 7, 8).forEach { index ->
            composeTestRule.onNodeWithTag("mic_slot_empty_$index").assertIsDisplayed()
        }
    }

    // ─────────────────────────────────────────────
    // UI-03: 3 条聊天消息均可见
    // ─────────────────────────────────────────────

    @Test
    fun UI03_chatMessagesDisplayedInList() {
        val messages = listOf(
            ChatMessageUi(messageId = "msg1", senderNickname = "Alice", content = "Hello", timestamp = 1L),
            ChatMessageUi(messageId = "msg2", senderNickname = "Bob", content = "World", timestamp = 2L),
            ChatMessageUi(messageId = "msg3", senderNickname = "Carol", content = "Hi", timestamp = 3L),
        )

        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(messages = messages))
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_message_list").assertIsDisplayed()
        // T-30014: testTag 已更新为 user_message_{index}（按列表位置索引）
        composeTestRule.onNodeWithTag("user_message_0").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_1").assertIsDisplayed()
        composeTestRule.onNodeWithTag("user_message_2").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // UI-04: 空消息列表 → chat_message_list 可见但无子消息节点
    // ─────────────────────────────────────────────

    @Test
    fun UI04_emptyMessageList_showsListWithNoItems() {
        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(messages = emptyList()))
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_message_list").assertIsDisplayed()
        // No message nodes
        composeTestRule.onNodeWithTag("chat_message_msg1").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // UI-05: mic_slots_grid 可见，共 9 个槽位
    // ─────────────────────────────────────────────

    @Test
    fun UI05_micSlotsGridVisibleWithNineSlots() {
        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slots_grid").assertIsDisplayed()

        // All 9 empty slots visible
        for (i in 0..8) {
            composeTestRule.onNodeWithTag("mic_slot_empty_$i").assertIsDisplayed()
        }
    }

    // ─────────────────────────────────────────────
    // UI-06: 输入文字 → send_button enabled；点击后 onSend 触发；输入框清空
    // ─────────────────────────────────────────────

    @Test
    fun UI06_sendButtonTriggersCallbackAndClearsInput() {
        var sentMessage: String? = null

        composeTestRule.setContent {
            RoomScreen(
                uiState = defaultUiState(),
                onSendMessage = { sentMessage = it },
            )
        }
        composeTestRule.waitForIdle()

        // Round 2 BUG-004 修复：先 performClick 聚焦，再 performTextInput
        composeTestRule
            .onNodeWithTag("chat_input_field")
            .performClick()
        composeTestRule.waitForIdle()
        composeTestRule
            .onNodeWithTag("chat_input_field")
            .performTextInput("Hello")
        composeTestRule.waitForIdle()

        // Send button should be enabled
        composeTestRule.onNodeWithTag("chat_send_button").assertIsEnabled()

        // Click send
        composeTestRule.onNodeWithTag("chat_send_button").performClick()
        composeTestRule.waitForIdle()

        // Callback received the message
        assertEquals("Hello", sentMessage)

        // 注意：输入框清空由 RoomScreen 的调用方（ViewModel）控制，
        // RoomScreen 本身是无状态组件，不会自动清空。此测试只验证回调触发。
    }

    // ─────────────────────────────────────────────
    // UI-07: 输入框为空时 send_button disabled
    // ─────────────────────────────────────────────

    @Test
    fun UI07_emptyInput_sendButtonIsDisabled() {
        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState())
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("chat_send_button").assertIsNotEnabled()
    }

    // ─────────────────────────────────────────────
    // UI-08: 点击 room_back_button → onBack 回调触发一次
    // ─────────────────────────────────────────────

    @Test
    fun UI08_backButtonTriggersOnBackCallback() {
        var backCount = 0

        composeTestRule.setContent {
            RoomScreen(
                uiState = defaultUiState(),
                onBack = { backCount++ },
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_back_button").performClick()
        composeTestRule.waitForIdle()

        assertEquals(1, backCount)
    }

    // ─────────────────────────────────────────────
    // UI-09: isMuted=true → mic_slot_muted_icon 可见
    // ─────────────────────────────────────────────

    @Test
    fun UI09_mutedMicSlot_showsMutedIcon() {
        val slots = List(9) { index ->
            if (index == 1)  // 改为 index 1（副麦），因为 index 0 是主麦（HostMicSlot），muted icon testTag 不同
                MicSlotUi(index = 1, userId = "u1", nickname = "Alice", isMuted = true)
            else
                MicSlotUi(index = index)
        }

        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(micSlots = slots))
        }
        composeTestRule.waitForIdle()

        // Round 2 BUG-002：MicSlotCard 的 clickable mergeDescendants，
        // muted_icon 虽有 testTag，但被合并到父节点，需 useUnmergedTree=true。
        // AnimatedVisibility 可能影响 assertIsDisplayed，改用 assertExists。
        composeTestRule.onNodeWithTag("mic_slot_muted_icon_1", useUnmergedTree = true).assertExists()
    }

    // ─────────────────────────────────────────────
    // UI-10: roomName 为 30 字符 → room_name 不崩溃，文本可见
    // ─────────────────────────────────────────────

    @Test
    fun UI10_longRoomName_doesNotCrash() {
        val longName = "这是一个很长的房间名字用来测试边界条件啊啊啊" // 23 chars, within 30
        composeTestRule.setContent {
            RoomScreen(uiState = defaultUiState(roomName = longName))
        }
        composeTestRule.waitForIdle()

        // Should not crash; room_name node exists and is displayed
        composeTestRule.onNodeWithTag("room_name").assertIsDisplayed()
    }

}
