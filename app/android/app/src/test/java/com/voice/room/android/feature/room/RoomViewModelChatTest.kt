package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * BUG-CHAT-WS Round 6 回归测试。
 *
 * 服务端（app/server/src/room/handler/chat.rs）实际广播的消息形如：
 *   {
 *     "type": "RoomMessage",
 *     "payload": { "msg_id": "...", "user_id": "...", "content": "..." },
 *     "timestamp": 1234
 *   }
 *
 * 但 RoomViewModel.handleWsMessage 仅处理 `type=="MessageReceived"` 且字段为顶层
 * `msgId/senderNickname/content`，导致房间内聊天发送后公屏不渲染。
 *
 * 本套测试覆盖 RoomMessage 信封解析、按 msg_id 去重、用户昵称查找。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomViewModelChatTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var viewModel: RoomViewModel

    private val snapshotWithMember = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 2,
        micSlots = listOf(
            MicSlotData(index = 0, userId = "user-7", nickname = "Alice"),
            MicSlotData(index = 1, userId = null, nickname = null),
        )
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(snapshotWithMember)
        fakeMediaService = FakeMediaService()
        viewModel = RoomViewModel(fakeWsClient, fakeRepo, fakeMediaService)
    }

    @Test
    fun `BUG-CHAT-WS RoomMessage envelope appended to chat messages`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"m-1","user_id":"user-7","content":"hello"},"timestamp":1700000000000}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("chatMessages should have 1 item", 1, state.uiState.messages.size)
            val msg = state.uiState.messages[0]
            assertEquals("messageId should match payload.msg_id", "m-1", msg.messageId)
            assertEquals("content should match payload.content", "hello", msg.content)
            assertEquals("timestamp should be propagated", 1700000000000L, msg.timestamp)
            assertNotNull("senderNickname should be resolved (or fallback)", msg.senderNickname)
        }

    @Test
    fun `BUG-CHAT-WS duplicate RoomMessage msg_id deduped`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"dup","user_id":"user-7","content":"first"},"timestamp":1}"""
            )
            advanceUntilIdle()
            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"dup","user_id":"user-7","content":"second"},"timestamp":2}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("duplicate msg_id should not append twice", 1, state.uiState.messages.size)
            assertEquals("first content retained", "first", state.uiState.messages[0].content)
        }

    @Test
    fun `BUG-CHAT-WS RoomMessage missing content silently ignored`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"bad","user_id":"user-7"},"timestamp":1}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("malformed RoomMessage should be ignored", 0, state.uiState.messages.size)
        }

    @Test
    fun `BUG-CHAT-WS self sent RoomMessage broadcast back is rendered`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 模拟服务端将发送方自己的消息也回流广播
            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"self-1","user_id":"user-7","content":"我自己发的"},"timestamp":42}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertTrue(
                "self echoed RoomMessage must be rendered (no client-side drop)",
                state.uiState.messages.any { it.messageId == "self-1" && it.content == "我自己发的" }
            )
        }
}
