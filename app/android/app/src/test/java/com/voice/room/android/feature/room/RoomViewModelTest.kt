package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.core.ws.RoomSocketRequestSpec
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.data.room.IRoomSnapshotRepository
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — RoomViewModel (T-30010)
 *
 * VM-01: joinRoom 成功 → uiState 从 Loading 变为 Success，roomName/micSlots 正确
 * VM-02: joinRoom HTTP 失败 → uiState 变为 Error，message 非空
 * VM-03: 收到 UserJoined WS 消息 → onlineCount+1
 * VM-04: 收到 UserLeft WS 消息 → onlineCount-1
 * VM-05: 收到 MicTaken → 对应 slot userId/nickname 更新
 * VM-06: 收到 MicLeft → 对应 slot userId=null
 * VM-07: 收到 MessageReceived → chatMessages 追加新消息
 * VM-08: 重复 msgId → 不追加（去重）
 * VM-09: 收到 RoomClosed → events 发出 NavigateBack
 * VM-10: leaveRoom() → wsClient.disconnect() 被调用
 * VM-11: onCleared() → leaveRoom() 被调用（disconnect 被触发）
 * VM-12: sendMessage() → wsClient.send 以正确 JSON 格式调用
 * TM-03B: startPublishAudio 失败 → ShowToast 错误提示（R1 HIGH fix）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var viewModel: RoomViewModel

    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 5,
        micSlots = listOf(
            MicSlotData(index = 0, userId = "user-0", nickname = "Nick0"),
            MicSlotData(index = 1, userId = null, nickname = null),
        )
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        fakeMediaService = FakeMediaService()
        viewModel = RoomViewModel(fakeWsClient, fakeRepo, fakeMediaService)
    }

    // ─── VM-01: joinRoom 成功 → uiState Success ────────────────────────────────

    @Test
    fun `VM-01 joinRoom success - uiState transitions to Success with correct data`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected Success state", state is RoomViewState.Success)

            val success = state as RoomViewState.Success
            assertEquals("roomName should match snapshot", "Test Room", success.uiState.roomName)
            assertEquals("onlineCount should match snapshot", 5, success.uiState.onlineCount)
            assertEquals("slot-0 userId should be user-0", "user-0", success.uiState.micSlots[0].userId)
            assertEquals("roomId should match", "room-1", success.uiState.roomId)
        }

    // ─── VM-02: joinRoom HTTP 失败 → uiState Error ─────────────────────────────

    @Test
    fun `VM-02 joinRoom HTTP failure - uiState transitions to Error with non-empty message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeRepo.throwError = RuntimeException("HTTP 500 Internal Server Error")

            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected Error state", state is RoomViewState.Error)
            val error = state as RoomViewState.Error
            assertTrue("Error message should not be empty", error.message.isNotEmpty())
        }

    // ─── VM-03: UserJoined → onlineCount+1 ────────────────────────────────────

    @Test
    fun `VM-03 WS UserJoined - onlineCount increases by 1`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage("""{"type":"UserJoined","payload":{"user_id":"u99","nickname":""}}""")
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("onlineCount should be 6 after UserJoined", 6, state.uiState.onlineCount)
        }

    // ─── VM-04: UserLeft → onlineCount-1 ──────────────────────────────────────

    @Test
    fun `VM-04 WS UserLeft - onlineCount decreases by 1`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage("""{"type":"UserLeft","payload":{"user_id":"user-0"}}""")
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("onlineCount should be 4 after UserLeft", 4, state.uiState.onlineCount)
        }

    // ─── VM-05: MicTaken → slot userId/nickname 更新 ───────────────────────────

    @Test
    fun `VM-05 WS MicTaken - corresponding slot userId and nickname updated`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"new-user","nickname":"NewNick"}}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            val slot = state.uiState.micSlots[1]
            assertEquals("slot-1 userId should be updated", "new-user", slot.userId)
            assertEquals("slot-1 nickname should be updated", "NewNick", slot.nickname)
        }

    // ─── VM-06: MicLeft → slot userId=null ────────────────────────────────────

    @Test
    fun `VM-06 WS MicLeft - corresponding slot userId becomes null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // slot-0 was occupied (user-0), MicLeft should clear it
            fakeWsClient.simulateMessage("""{"type":"MicLeft","payload":{"mic_index":0}}""")
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            val slot = state.uiState.micSlots[0]
            assertNull("slot-0 userId should be null after MicLeft", slot.userId)
        }

    // ─── VM-07: MessageReceived → chatMessages 追加 ────────────────────────────

    @Test
    fun `VM-07 WS MessageReceived - message appended to chatMessages`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"msg-1","senderNickname":"Alice","content":"Hello","timestamp":1000}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("chatMessages should have 1 item", 1, state.uiState.messages.size)
            val msg = state.uiState.messages[0]
            assertEquals("msgId should match", "msg-1", msg.messageId)
            assertEquals("senderNickname should match", "Alice", msg.senderNickname)
            assertEquals("content should match", "Hello", msg.content)
            assertEquals("timestamp should match", 1000L, msg.timestamp)
        }

    // ─── VM-08: 重复 msgId → 不追加 ───────────────────────────────────────────

    @Test
    fun `VM-08 duplicate msgId - message not appended again`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 第一条消息
            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"msg-dup","senderNickname":"Bob","content":"Original","timestamp":1000}"""
            )
            advanceUntilIdle()

            // 相同 msgId，不同 content/timestamp（StateFlow 会发射因为值不同）
            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"msg-dup","senderNickname":"Bob","content":"Duplicate","timestamp":2000}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals(
                "chatMessages should still have only 1 item after duplicate msgId",
                1,
                state.uiState.messages.size
            )
            assertEquals("Only the first message should be retained", "Original", state.uiState.messages[0].content)
        }

    // ─── VM-09: RoomClosed → NavigateBack event ────────────────────────────────

    @Test
    fun `VM-09 WS RoomClosed - NavigateBack event emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch {
                viewModel.events.collect { collectedEvents.add(it) }
            }

            fakeWsClient.simulateMessage("""{"type":"RoomClosed"}""")
            advanceUntilIdle()

            assertTrue(
                "NavigateBack event should be emitted on RoomClosed",
                collectedEvents.contains(RoomEvent.NavigateBack)
            )
            collectJob.cancel()
        }

    // ─── VM-10: leaveRoom() → wsClient.disconnect() 被调用 ────────────────────

    @Test
    fun `VM-10 leaveRoom - wsClient disconnect is called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 先 connect 确保状态为 Connected
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))

            viewModel.leaveRoom()

            assertTrue(
                "WS state should be Disconnected after leaveRoom",
                fakeWsClient.state.value is WebSocketState.Disconnected
            )
            assertEquals(
                "Disconnect reason should be 'manual'",
                "manual",
                (fakeWsClient.state.value as WebSocketState.Disconnected).reason
            )
        }

    // ─── VM-11: onCleared() → leaveRoom() 被调用 ──────────────────────────────

    @Test
    fun `VM-11 onCleared - leaveRoom is called which triggers disconnect`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // onCleared() 已恢复 protected，通过 @VisibleForTesting 的 triggerOnCleared() 间接调用
            viewModel.triggerOnCleared()

            assertTrue(
                "WS state should be Disconnected after onCleared",
                fakeWsClient.state.value is WebSocketState.Disconnected
            )
            assertEquals(
                "Disconnect reason should be 'manual'",
                "manual",
                (fakeWsClient.state.value as WebSocketState.Disconnected).reason
            )
        }

    // ─── MP-08: onMicPermissionGranted 无活跃房间时静默不崩溃 ─────────────────

    @Test
    fun `MP-08 onMicPermissionGranted with no active room does nothing`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val events = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { events.add(it) } }

            // 未调用 joinRoom，currentRoomId 为 null → 应静默返回
            viewModel.onMicPermissionGranted(slotIndex = 2)
            advanceUntilIdle()

            assertTrue("No events should be emitted when no active room", events.isEmpty())
            assertTrue("No WS messages should be sent", fakeWsClient.sentMessages.isEmpty())
            job.cancel()
        }

    // ─── TM-01: onMicPermissionGranted → 发送 TakeMic WS 消息 ────────────────

    @Test
    fun `TM-01 onMicPermissionGranted sends TakeMic WS message with roomId and slotIndex`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // connect 使 FakeWebSocketClient 进入 Connected 状态，send() 才会入队
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.onMicPermissionGranted(slotIndex = 1)
            advanceUntilIdle()

            assertEquals("Should send exactly 1 message", 1, fakeWsClient.sentMessages.size)
            val sent = fakeWsClient.sentMessages[0]
            assertTrue("""Should contain "type":"TakeMic"""", sent.contains(""""type":"TakeMic""""))
            assertTrue("""Payload should carry mic_index=1""", sent.contains(""""mic_index":1"""))
        }

    // ─── TM-02: 收到 MicTaken（自己）→ joinChannel 被调用 ─────────────────────

    @Test
    fun `TM-02 WS MicTaken for currentUser - mediaService joinChannel called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            assertEquals(
                "joinChannel should be called once",
                1,
                fakeMediaService.joinChannelCalls.size
            )
            assertEquals(
                "joinChannel roomId should match",
                "room-1",
                fakeMediaService.joinChannelCalls[0].first
            )
            assertEquals(
                "joinChannel userId should match",
                "me",
                fakeMediaService.joinChannelCalls[0].second
            )
        }

    // ─── TM-03: 收到 MicTaken（自己）→ startPublishAudio 被调用 ───────────────

    @Test
    fun `TM-03 WS MicTaken for currentUser - mediaService startPublishAudio called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            assertEquals(
                "startPublishAudio should be called once",
                1,
                fakeMediaService.startPublishAudioCalls.size
            )
        }

    // ─── TM-03B: startPublishAudio 失败 → ShowToast 错误提示 ──────────────────

    @Test
    fun `TM-03B startPublishAudio failure - ShowToast error event emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeMediaService.startPublishAudioResult =
                Result.failure(RuntimeException("publish stream failed"))

            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            val events = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { events.add(it) } }

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            assertTrue(
                "ShowToast error event should be emitted on startPublishAudio failure",
                events.any { it is RoomEvent.ShowToast }
            )
            val toastMsg = events.filterIsInstance<RoomEvent.ShowToast>().first().message
            assertTrue(
                "Toast message should contain failure info",
                toastMsg.contains("publish stream failed", ignoreCase = true) ||
                    toastMsg.contains("推流", ignoreCase = false)
            )
            job.cancel()
        }

    // ─── TM-04: onMicSlotClick 点击自己的麦位 → 发送 LeaveMic ─────────────────

    @Test
    fun `TM-04 onMicSlotClick own occupied slot - sends LeaveMic WS message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // slot-0 被 "user-0" 占用（来自 defaultSnapshot），connect 进入 Connected 状态
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1", userId = "user-0")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.onMicSlotClick(slotIndex = 0)
            advanceUntilIdle()

            assertEquals("Should send exactly 1 message", 1, fakeWsClient.sentMessages.size)
            val sent = fakeWsClient.sentMessages[0]
            assertTrue("""Should contain "type":"LeaveMic"""", sent.contains(""""type":"LeaveMic""""))
            // P0-1: LeaveMic 由 server 通过连接上下文推断 room/mic，无需在 payload 中携带
            assertTrue("""Envelope must carry empty payload object""", sent.contains(""""payload":{}"""))
        }

    // ─── TM-05: 收到 MicLeft（自己）→ stopPublishAudio 被调用 ─────────────────

    @Test
    fun `TM-05 WS MicLeft for currentUser slot - mediaService stopPublishAudio called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // slot-0 被 "user-0" 占用，currentUser = "user-0"
            viewModel.joinRoom("room-1", userId = "user-0")
            advanceUntilIdle()

            fakeWsClient.simulateMessage("""{"type":"MicLeft","payload":{"mic_index":0}}""")
            advanceUntilIdle()

            assertEquals(
                "stopPublishAudio should be called once",
                1,
                fakeMediaService.stopPublishAudioCalls.size
            )
        }

    // ─── TM-06: 收到 MicLeft（自己）→ leaveChannel 被调用 ────────────────────

    @Test
    fun `TM-06 WS MicLeft for currentUser slot - mediaService leaveChannel called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "user-0")
            advanceUntilIdle()

            fakeWsClient.simulateMessage("""{"type":"MicLeft","payload":{"mic_index":0}}""")
            advanceUntilIdle()

            assertEquals(
                "leaveChannel should be called once",
                1,
                fakeMediaService.leaveChannelCalls.size
            )
        }

    // ─── TM-07: onMicSlotClick 点击他人麦位 → 不操作 ─────────────────────────

    @Test
    fun `TM-07 onMicSlotClick other user's slot - no LeaveMic sent, no mediaService call`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // slot-0 被 "user-0" 占用，当前用户是 "me"（他人的麦位）
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.onMicSlotClick(slotIndex = 0)
            advanceUntilIdle()

            assertTrue("No WS messages should be sent", fakeWsClient.sentMessages.isEmpty())
            assertTrue(
                "stopPublishAudio should NOT be called",
                fakeMediaService.stopPublishAudioCalls.isEmpty()
            )
            assertTrue(
                "leaveChannel should NOT be called",
                fakeMediaService.leaveChannelCalls.isEmpty()
            )
        }

    // ─── TM-08: joinChannel 失败 → ShowToast 错误提示 ─────────────────────────

    @Test
    fun `TM-08 joinChannel failure - ShowToast error event emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeMediaService.joinChannelResult = Result.failure(RuntimeException("RTC connection refused"))

            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            val events = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { events.add(it) } }

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            assertTrue(
                "ShowToast error event should be emitted on joinChannel failure",
                events.any { it is RoomEvent.ShowToast }
            )
            val toastMsg = events.filterIsInstance<RoomEvent.ShowToast>().first().message
            assertTrue(
                "Toast message should contain error info",
                toastMsg.isNotEmpty()
            )
            job.cancel()
        }

    // ─── SM-01: sendMessage("") → 不发送，无 ClearInput 事件 ──────────────────

    @Test
    fun `SM-01 sendMessage blank content - wsClient not called, no ClearInput event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch { viewModel.events.collect { collectedEvents.add(it) } }

            viewModel.sendMessage("")
            advanceUntilIdle()

            assertTrue("wsClient should NOT be called for blank content", fakeWsClient.sentMessages.isEmpty())
            assertFalse(
                "ClearInput event should NOT be emitted for blank content",
                collectedEvents.any { it is RoomEvent.ClearInput }
            )

            // 同样测试空白字符（whitespace-only）
            viewModel.sendMessage("   ")
            advanceUntilIdle()

            assertTrue("wsClient should NOT be called for whitespace content", fakeWsClient.sentMessages.isEmpty())
            collectJob.cancel()
        }

    // ─── SM-02: sendMessage("hello") → wsClient 发送正确 JSON ────────────────

    @Test
    fun `SM-02 sendMessage valid content - wsClient sends correct JSON with type roomId content msgId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            assertEquals("Should send exactly 1 WS message", 1, fakeWsClient.sentMessages.size)
            val sent = fakeWsClient.sentMessages[0]
            assertTrue("""JSON must contain "type":"SendMessage"""", sent.contains(""""type":"SendMessage""""))
            assertTrue("""JSON must contain "content":"hello"""", sent.contains(""""content":"hello""""))
            assertTrue("Envelope must contain a non-empty msg_id", sent.contains(""""msg_id":"""))
        }

    // ─── SM-03: 发送成功 → ClearInput 事件发出 ───────────────────────────────

    @Test
    fun `SM-03 sendMessage success - ClearInput event emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch { viewModel.events.collect { collectedEvents.add(it) } }

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            assertTrue(
                "ClearInput event should be emitted after successful send",
                collectedEvents.any { it is RoomEvent.ClearInput }
            )
            collectJob.cancel()
        }

    // ─── SM-04: 发送过程 isSendingMessage=true，完成后=false ─────────────────

    @Test
    fun `SM-04 isSendingMessage transitions true during send then false after completion`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 使用 UnconfinedTestDispatcher 收集器：StateFlow 每次变更时立刻在线收集
            val isSendingHistory = mutableListOf<Boolean>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                viewModel.uiState.collect { state ->
                    (state as? RoomViewState.Success)?.let {
                        isSendingHistory.add(it.uiState.isSendingMessage)
                    }
                }
            }
            advanceUntilIdle()  // 让收集器先稳定在初始值 false

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            assertTrue(
                "isSendingMessage should have been true during send. History: $isSendingHistory",
                isSendingHistory.contains(true)
            )
            assertFalse(
                "isSendingMessage should be false after send completes. History: $isSendingHistory",
                isSendingHistory.last()
            )
            collectJob.cancel()
        }

    // ─── SM-05: wsClient.send 抛异常 → ShowToast，不发 ClearInput ─────────────

    @Test
    fun `SM-05 wsClient send throws exception - ShowToast emitted, ClearInput NOT emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 注入发送异常
            fakeWsClient.sendThrowable = RuntimeException("Network write failed")

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch { viewModel.events.collect { collectedEvents.add(it) } }

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            assertTrue(
                "ShowToast event should be emitted on send failure",
                collectedEvents.any { it is RoomEvent.ShowToast }
            )
            assertFalse(
                "ClearInput event should NOT be emitted on send failure",
                collectedEvents.any { it is RoomEvent.ClearInput }
            )
            val toastMsg = collectedEvents.filterIsInstance<RoomEvent.ShowToast>().first().message
            assertTrue("Toast message should not be blank", toastMsg.isNotBlank())
            collectJob.cancel()
        }

    // ─── SM-06: 发送失败后再次 sendMessage → 可正常发送 ───────────────────────

    @Test
    fun `SM-06 sendMessage after failure - retry succeeds without permanent isSending lock`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 第一次发送失败
            fakeWsClient.sendThrowable = RuntimeException("Network write failed")
            viewModel.sendMessage("first attempt")
            advanceUntilIdle()

            // 重置异常注入（第二次正常发送）
            fakeWsClient.sendThrowable = null
            fakeWsClient.sentMessages.clear()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch { viewModel.events.collect { collectedEvents.add(it) } }

            viewModel.sendMessage("retry")
            advanceUntilIdle()

            assertEquals("Retry should send exactly 1 WS message", 1, fakeWsClient.sentMessages.size)
            assertTrue(
                "ClearInput should be emitted on successful retry",
                collectedEvents.any { it is RoomEvent.ClearInput }
            )
            // isSendingMessage 最终为 false，不被永久锁定
            val finalState = viewModel.uiState.value as? RoomViewState.Success
            assertFalse(
                "isSendingMessage should be false after retry completes",
                finalState?.uiState?.isSendingMessage ?: true
            )
            collectJob.cancel()
        }



    // ─── RM-03: 字段解析完整性 ────────────────────────────────────────────────
    //  T-30017: 消息内容/昵称/timestamp 正确映射到 ChatMessageUi 各字段

    @Test
    fun `RM-03 WS MessageReceived - all fields correctly parsed into ChatMessageUi`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"rm03-id","senderNickname":"Bob","content":"Hi there","timestamp":9876543210}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("Should have exactly 1 message", 1, state.uiState.messages.size)
            val msg = state.uiState.messages[0]
            assertEquals("messageId should match msgId", "rm03-id", msg.messageId)
            assertEquals("senderNickname should match", "Bob", msg.senderNickname)
            assertEquals("content should match", "Hi there", msg.content)
            assertEquals("timestamp should match", 9876543210L, msg.timestamp)
            assertEquals("messageType should default to USER_TEXT", MessageType.USER_TEXT, msg.messageType)
        }

    // ─── RM-04: 多条消息按顺序追加 ────────────────────────────────────────────
    //  T-30017: 收到多条 MessageReceived → 按接收顺序追加，顺序不乱

    @Test
    fun `RM-04 multiple MessageReceived - appended in order`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"m-001","senderNickname":"Alice","content":"First","timestamp":1000}"""
            )
            advanceUntilIdle()
            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"m-002","senderNickname":"Bob","content":"Second","timestamp":2000}"""
            )
            advanceUntilIdle()
            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"m-003","senderNickname":"Carol","content":"Third","timestamp":3000}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            val msgs = state.uiState.messages
            assertEquals("Should have 3 messages", 3, msgs.size)
            assertEquals("First message content should be 'First'", "First", msgs[0].content)
            assertEquals("Second message content should be 'Second'", "Second", msgs[1].content)
            assertEquals("Third message content should be 'Third'", "Third", msgs[2].content)
            assertEquals("Message IDs should be in order", listOf("m-001", "m-002", "m-003"), msgs.map { it.messageId })
        }

    // ─── RM-05: content 缺失的非法消息 → 静默忽略 ────────────────────────────
    //  T-30017: content 字段缺失时不追加消息，不崩溃

    @Test
    fun `RM-05 MessageReceived missing content field - silently ignored, no message appended`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // content 字段缺失的非法消息
            fakeWsClient.simulateMessage(
                """{"type":"MessageReceived","msgId":"rm05-bad","senderNickname":"Alice","timestamp":1000}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals(
                "messages should remain empty when content is missing",
                0,
                state.uiState.messages.size
            )
        }

    @Test
    fun `VM-12 sendMessage - wsClient send called with correct JSON format`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 先 connect 保证 Connected 状态，send() 才会入队
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 清空 joinRoom 时产生的 JoinRoom 消息，聚焦 sendMessage 的结果
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("Hello World")
            advanceUntilIdle()  // sendMessage 改为异步，需等待协程完成

            assertEquals("Should have sent 1 message", 1, fakeWsClient.sentMessages.size)
            val sent = fakeWsClient.sentMessages[0]
            assertTrue("""JSON should contain "type":"SendMessage"""", sent.contains(""""type":"SendMessage""""))
            assertTrue("""JSON should contain "content":"Hello World"""", sent.contains(""""content":"Hello World""""))
            assertTrue("Envelope should contain a msg_id field", sent.contains(""""msg_id":"""))
        }

    // ─── MT-01: 不在麦上时 toggleMicMute() 无效 ──────────────────────────────

    @Test
    fun `MT-01 toggleMicMute when not on mic - isCurrentUserMuted unchanged, mediaService not called`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            // 确认初始不在麦上
            val before = viewModel.uiState.value as RoomViewState.Success
            assertFalse("isCurrentUserOnMic should be false initially", before.uiState.isCurrentUserOnMic)

            viewModel.toggleMicMute()
            advanceUntilIdle()

            val after = viewModel.uiState.value as RoomViewState.Success
            assertFalse("isCurrentUserMuted should remain false", after.uiState.isCurrentUserMuted)
            assertTrue("stopPublishAudio should NOT be called", fakeMediaService.stopPublishAudioCalls.isEmpty())
            assertTrue("startPublishAudio should NOT be called (for toggleMicMute)", fakeMediaService.startPublishAudioCalls.isEmpty())
        }

    // ─── MT-02: 在麦上 + 未静音 → stopPublishAudio，isCurrentUserMuted=true ────

    @Test
    fun `MT-02 toggleMicMute when on mic and not muted - stopPublishAudio called and isCurrentUserMuted becomes true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            // 模拟自己上麦，令 isCurrentUserOnMic = true, isCurrentUserMuted = false
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            val before = viewModel.uiState.value as RoomViewState.Success
            assertTrue("Should be on mic", before.uiState.isCurrentUserOnMic)
            assertFalse("Should not be muted initially", before.uiState.isCurrentUserMuted)

            // 清空 startPublishAudio 调用记录（上麦时会调用一次）
            fakeMediaService.stopPublishAudioCalls.clear()

            viewModel.toggleMicMute()
            advanceUntilIdle()

            assertEquals("stopPublishAudio should be called once", 1, fakeMediaService.stopPublishAudioCalls.size)
            val after = viewModel.uiState.value as RoomViewState.Success
            assertTrue("isCurrentUserMuted should become true", after.uiState.isCurrentUserMuted)
        }

    // ─── MT-03: 在麦上 + 已静音 → startPublishAudio，isCurrentUserMuted=false ──

    @Test
    fun `MT-03 toggleMicMute when on mic and already muted - startPublishAudio called and isCurrentUserMuted becomes false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            // 上麦
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            // 先调一次 toggleMicMute 令其静音
            fakeMediaService.stopPublishAudioCalls.clear()
            viewModel.toggleMicMute()
            advanceUntilIdle()

            val muted = viewModel.uiState.value as RoomViewState.Success
            assertTrue("Should be muted now", muted.uiState.isCurrentUserMuted)

            // 再次 toggle → 取消静音
            fakeMediaService.startPublishAudioCalls.clear()
            viewModel.toggleMicMute()
            advanceUntilIdle()

            assertEquals("startPublishAudio should be called once", 1, fakeMediaService.startPublishAudioCalls.size)
            val after = viewModel.uiState.value as RoomViewState.Success
            assertFalse("isCurrentUserMuted should become false again", after.uiState.isCurrentUserMuted)
        }

    // ─── MT-04: toggleMicMute() 中 stopPublishAudio 抛异常 → ShowToast，状态不变 ─

    @Test
    fun `MT-04 toggleMicMute stopPublishAudio throws - ShowToast emitted and isCurrentUserMuted unchanged`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            // 注入 stopPublishAudio 失败
            fakeMediaService.stopPublishAudioResult = Result.failure(RuntimeException("mic hardware error"))
            fakeMediaService.stopPublishAudioCalls.clear()

            val events = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { events.add(it) } }

            viewModel.toggleMicMute()
            advanceUntilIdle()

            assertTrue(
                "ShowToast event should be emitted on stopPublishAudio failure",
                events.any { it is RoomEvent.ShowToast }
            )
            val toastMsg = events.filterIsInstance<RoomEvent.ShowToast>().first().message
            assertTrue("Toast message should not be blank", toastMsg.isNotBlank())

            val after = viewModel.uiState.value as RoomViewState.Success
            assertFalse("isCurrentUserMuted should remain false after failure", after.uiState.isCurrentUserMuted)

            job.cancel()
        }

    // ─── MT-05: CancellationException 在 toggleMicMute() 中被 re-throw ─────────

    @Test
    fun `MT-05 toggleMicMute CancellationException is re-thrown not swallowed`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"me","nickname":"MyNick"}}"""
            )
            advanceUntilIdle()

            // 注入 CancellationException
            fakeMediaService.stopPublishAudioResult =
                Result.failure(kotlinx.coroutines.CancellationException("coroutine cancelled"))

            val events = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { events.add(it) } }

            // CancellationException 不应被 ShowToast 吞掉
            viewModel.toggleMicMute()
            advanceUntilIdle()

            assertFalse(
                "ShowToast should NOT be emitted for CancellationException",
                events.any { it is RoomEvent.ShowToast }
            )
            job.cancel()
        }

    // ─── MT-06: WS MicTaken（自己）→ isCurrentUserOnMic=true, isCurrentUserMuted=false ─

    @Test
    fun `MT-06 WS MicTaken for self - isCurrentUserOnMic becomes true and isCurrentUserMuted reset to false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":2,"user_id":"me","nickname":"Me"}}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertTrue("isCurrentUserOnMic should be true after MicTaken", state.uiState.isCurrentUserOnMic)
            assertFalse("isCurrentUserMuted should be false after MicTaken", state.uiState.isCurrentUserMuted)
        }

    // ─── MT-07: WS MicLeft（自己）→ isCurrentUserOnMic=false, isCurrentUserMuted=false ─

    @Test
    fun `MT-07 WS MicLeft for self - isCurrentUserOnMic becomes false and isCurrentUserMuted reset to false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 先让自己上麦
            viewModel.joinRoom("room-1", userId = "me")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":2,"user_id":"me","nickname":"Me"}}"""
            )
            advanceUntilIdle()

            val onMic = viewModel.uiState.value as RoomViewState.Success
            assertTrue("Should be on mic before MicLeft", onMic.uiState.isCurrentUserOnMic)

            // 再触发下麦事件
            fakeWsClient.simulateMessage("""{"type":"MicLeft","payload":{"mic_index":2}}""")
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertFalse("isCurrentUserOnMic should be false after MicLeft", state.uiState.isCurrentUserOnMic)
            assertFalse("isCurrentUserMuted should be false after MicLeft", state.uiState.isCurrentUserMuted)
        }

    // ─── VM-GR1: GiftReceived 协议字段 gift.id 能正确解析，触发 giftMessages ──
    // [RED] 当前代码使用 giftObj.get("giftId") 而非 giftObj.get("id")，
    //       收到协议正确的 JSON（含 "id" 字段）时 giftId 解析为 null → ?: return
    //       → giftMessages.value 仍为空。测试断言 size==1 将 FAIL。
    // 修复 RoomViewModel 改用 "id" / "code" 后，此测试变为 GREEN。

    @Test
    fun `VM-GR1 GiftReceived with protocol-correct gift dot id field triggers giftMessages`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // §6.4.3 协议：gift 对象字段名为 "id" 和 "code"（非 "giftId"/"giftCode"）
            fakeWsClient.simulateMessage(
                """{"type":"GiftReceived","msgId":"gr1-msg","giftRecordId":"rec-gr1",""" +
                """"sender":{"userId":"sender-1","nickname":"Alice","avatar":null},""" +
                """"receiver":{"userId":"receiver-1","nickname":"Bob","avatar":null},""" +
                """"gift":{"id":"gift-uuid-1","code":"castle_01","name":"城堡",""" +
                """"icon_url":"https://icon.png","animation_url":null,"effect_level":1},""" +
                """"count":1,"totalPrice":10}"""
            )
            advanceUntilIdle()

            assertEquals(
                "giftMessages 应有 1 条弹幕（gift.id 字段能被正确解析）",
                1,
                viewModel.giftMessages.value.size,
            )
            assertEquals(
                "giftMessages[0].giftId 应等于协议中 gift.id 的值",
                "gift-uuid-1",
                viewModel.giftMessages.value[0].giftId,
            )
        }

    // ─── VM-GR2: GiftReceived giftCode 字段正确解析 ──────────────────────────
    // [RED] giftObj.get("giftCode") 返回 null → 降级为 ""；但 giftObj.get("code")
    //       应能拿到正确值。修复后 giftCode 字段映射正确。

    @Test
    fun `VM-GR2 GiftReceived with protocol-correct gift dot code field parses giftCode correctly`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"GiftReceived","msgId":"gr2-msg","giftRecordId":"rec-gr2",""" +
                """"sender":{"userId":"sender-1","nickname":"Alice","avatar":null},""" +
                """"receiver":{"userId":"receiver-1","nickname":"Bob","avatar":null},""" +
                """"gift":{"id":"gift-uuid-2","code":"bouquet_01","name":"花束",""" +
                """"icon_url":"https://icon2.png","animation_url":null,"effect_level":1},""" +
                """"count":2,"totalPrice":20}"""
            )
            advanceUntilIdle()

            assertEquals(
                "giftMessages 应有 1 条弹幕",
                1,
                viewModel.giftMessages.value.size,
            )
            // count 也应正确解析
            assertEquals(
                "count 应解析为 2",
                2,
                viewModel.giftMessages.value[0].count,
            )
        }

    // ─── UA40-07: 任命管理员 → ShowConfirmAssignAdmin 事件 ─────────────────────

    @Test
    fun `UA40-07 assignAdmin - emits ShowConfirmAssignAdmin event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val events = mutableListOf<RoomEvent>()
            val job = launch(UnconfinedTestDispatcher()) {
                viewModel.events.collect { events.add(it) }
            }

            viewModel.assignAdmin(targetUserId = "user-target")
            advanceUntilIdle()

            assertTrue(
                "assignAdmin should emit ShowConfirmAssignAdmin event",
                events.any { it is RoomEvent.ShowConfirmAssignAdmin && it.targetUserId == "user-target" }
            )
            job.cancel()
        }

    // ─── UA40-08: 踢出 → selectedKickTarget 被设置 ─────────────────────────────

    @Test
    fun `UA40-08 onKickAction - selectedKickTarget state is set to target member`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val targetMember = com.voice.room.android.data.model.RoomMember(
                id = "kick-user",
                nickname = "KickTarget",
                role = "member",
            )

            viewModel.onKickAction(targetMember)
            advanceUntilIdle()

            assertEquals(
                "selectedKickTarget should be set to the target member",
                targetMember,
                viewModel.selectedKickTarget.value,
            )
        }

    // ─── UA40-09: 禁麦 30min → WS payload duration_sec=1800 ───────────────────

    @Test
    fun `UA40-09 muteUser with 30min - WS sends MuteUser with duration_sec 1800`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.muteUser(
                targetUserId = "mute-user",
                durationSec = 1800,
                muteType = "mic",
            )
            advanceUntilIdle()

            val sentMessages = fakeWsClient.sentMessages
            assertTrue(
                "Should have sent a MuteUser WS message",
                sentMessages.any { it.contains("\"type\":\"MuteUser\"") }
            )
            val muteMsg = sentMessages.first { it.contains("\"type\":\"MuteUser\"") }
            assertTrue("MuteUser payload should contain target_user_id", muteMsg.contains("\"target_user_id\":\"mute-user\""))
            assertTrue("MuteUser payload should contain duration_sec=1800", muteMsg.contains("\"duration_sec\":1800"))
            // P0-1: server expects payload.type (not muteType)
            assertTrue("MuteUser payload should contain type=mic", muteMsg.contains("\"type\":\"mic\""))
        }

    // ─── UA40-09b: revokeAdmin → ShowConfirmRevokeAdmin 事件（R1 修复）─────────

    @Test
    fun `UA40-09b revokeAdmin - emits ShowConfirmRevokeAdmin event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val events = mutableListOf<RoomEvent>()
            val job = launch(UnconfinedTestDispatcher()) {
                viewModel.events.collect { events.add(it) }
            }

            viewModel.revokeAdmin(targetUserId = "admin-user")
            advanceUntilIdle()

            assertTrue(
                "revokeAdmin should emit ShowConfirmRevokeAdmin event",
                events.any { it is RoomEvent.ShowConfirmRevokeAdmin && it.targetUserId == "admin-user" }
            )
            // P0-1: AssignAdmin/RevokeAdmin 客户端类型已被合并为 server 端 TransferAdmin（action=assign/revoke）
            assertTrue(
                "revokeAdmin should NOT send WS before confirmation (no TransferAdmin envelope yet)",
                fakeWsClient.sentMessages.none { it.contains("\"type\":\"TransferAdmin\"") }
            )
            job.cancel()
        }

    // ─── UA40-09b-confirm: confirmRevokeAdmin → WS sends RevokeAdmin ─────────

    @Test
    fun `UA40-09b-confirm confirmRevokeAdmin - WS sends RevokeAdmin with targetUserId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.confirmRevokeAdmin(targetUserId = "admin-user")
            advanceUntilIdle()

            val sentMessages = fakeWsClient.sentMessages
            // P0-1: 客户端 RevokeAdmin → server 接受 TransferAdmin（action=revoke）
            assertTrue(
                "confirmRevokeAdmin should send TransferAdmin WS message with action=revoke",
                sentMessages.any {
                    it.contains("\"type\":\"TransferAdmin\"") &&
                        it.contains("admin-user") &&
                        it.contains("\"action\":\"revoke\"")
                }
            )
        }

    // ─── UA40-09c: forceTakeMic → WS sends ForceTakeMic ───────────────────────

    @Test
    fun `UA40-09c forceTakeMic - WS sends ForceTakeMic with targetUserId and slotIndex`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.forceTakeMic(targetUserId = "target-user", slotIndex = 3)
            advanceUntilIdle()

            val sentMessages = fakeWsClient.sentMessages
            assertTrue(
                "Should send ForceTakeMic WS message",
                sentMessages.any {
                    it.contains("\"type\":\"ForceTakeMic\"") &&
                        it.contains("\"target_user_id\":\"target-user\"") &&
                        it.contains("\"slot_index\":3")
                }
            )
        }

    // ─── UA40-09d: forceLeaveMic → WS sends ForceLeaveMic ─────────────────────

    @Test
    fun `UA40-09d forceLeaveMic - WS sends ForceLeaveMic with targetUserId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.forceLeaveMic(targetUserId = "target-mic-user")
            advanceUntilIdle()

            val sentMessages = fakeWsClient.sentMessages
            assertTrue(
                "Should send ForceLeaveMic WS message",
                sentMessages.any {
                    it.contains("\"type\":\"ForceLeaveMic\"") &&
                        it.contains("target-mic-user")
                }
            )
        }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-05: kickUser 成功 → ShowToast "已踢出"
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-05 kickUser success - emits ShowToast 已踢出`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch(UnconfinedTestDispatcher()) {
                viewModel.events.collect { collectedEvents.add(it) }
            }

            fakeWsClient.sentMessages.clear()
            viewModel.kickUser("target-user-1", "harassment")
            advanceUntilIdle()

            assertTrue(
                "kickUser should emit ShowToast '已踢出' on success",
                collectedEvents.any { it is RoomEvent.ShowToast && it.message == "已踢出" }
            )
            collectJob.cancel()
        }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-06: 收到 WS Error code=40301 → ShowToast "无权操作"
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-06 WS Error code 40301 - emits ShowToast 无权操作`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch(UnconfinedTestDispatcher()) {
                viewModel.events.collect { collectedEvents.add(it) }
            }

            fakeWsClient.simulateMessage("""{"type":"Error","code":40301}""")
            advanceUntilIdle()

            assertTrue(
                "WS Error 40301 should emit ShowToast '无权操作'",
                collectedEvents.any { it is RoomEvent.ShowToast && it.message == "无权操作" }
            )
            collectJob.cancel()
        }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-07: kickUser 发送正确 reason（预设 key / Other → customText）
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-07 kickUser with preset reason - WS message contains reason key`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.kickUser("target-user-1", "harassment")
            advanceUntilIdle()

            val kickMsg = fakeWsClient.sentMessages.firstOrNull {
                it.contains(""""type":"KickUser"""")
            }
            assertFalse("Should send KickUser WS message", kickMsg == null)
            assertTrue(
                "reason field should be 'harassment'",
                kickMsg!!.contains(""""reason":"harassment"""")
            )
            assertTrue(
                "target_user_id should be correct (snake_case)",
                kickMsg.contains(""""target_user_id":"target-user-1"""")
            )
        }

    @Test
    fun `KR41-07 kickUser with Other reason - WS message contains custom text`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.kickUser("target-user-2", "custom reason text")
            advanceUntilIdle()

            val kickMsg = fakeWsClient.sentMessages.firstOrNull {
                it.contains(""""type":"KickUser"""")
            }
            assertFalse("Should send KickUser WS message", kickMsg == null)
            assertTrue(
                "reason field should be 'custom reason text'",
                kickMsg!!.contains(""""reason":"custom reason text"""")
            )
        }

    @Test
    fun `KR41-05 kickUser clears selectedKickTarget on success`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            viewModel.kickUser("target-user-1", "spam")
            advanceUntilIdle()

            assertNull(
                "selectedKickTarget should be null after successful kick",
                viewModel.selectedKickTarget.value
            )
        }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-07b: reason 含双引号 / 反斜杠时 WS 消息仍是有效 JSON（R1 HIGH fix）
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-07b kickUser reason with double-quote - WS message is valid JSON`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            // reason 含双引号，直接拼接会破坏 JSON
            viewModel.kickUser("target-user-1", """she said "hello" to me""")
            advanceUntilIdle()

            val kickMsg = fakeWsClient.sentMessages.firstOrNull {
                it.contains(""""type":"KickUser"""")
            }
            assertFalse("Should send KickUser WS message", kickMsg == null)

            // 验证 WS 消息是合法 JSON（用 Gson 解析，非 Android 存根）
            val jsonElement = try {
                com.google.gson.JsonParser.parseString(kickMsg!!)
            } catch (e: com.google.gson.JsonParseException) {
                null
            }
            assertTrue(
                "WS message must be valid JSON even when reason contains double-quotes; got: $kickMsg",
                jsonElement != null && jsonElement.isJsonObject
            )
            assertEquals(
                "reason field must preserve the original text (under payload)",
                """she said "hello" to me""",
                jsonElement!!.asJsonObject.getAsJsonObject("payload").get("reason").asString
            )
        }

    @Test
    fun `KR41-07b kickUser reason with backslash - WS message is valid JSON`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            // reason 含反斜杠，直接拼接会破坏 JSON
            viewModel.kickUser("target-user-2", """path\to\file""")
            advanceUntilIdle()

            val kickMsg = fakeWsClient.sentMessages.firstOrNull {
                it.contains(""""type":"KickUser"""")
            }
            assertFalse("Should send KickUser WS message", kickMsg == null)

            val jsonElement = try {
                com.google.gson.JsonParser.parseString(kickMsg!!)
            } catch (e: com.google.gson.JsonParseException) {
                null
            }
            assertTrue(
                "WS message must be valid JSON even when reason contains backslash; got: $kickMsg",
                jsonElement != null && jsonElement.isJsonObject
            )
            assertEquals(
                "reason field must preserve the original text (under payload)",
                """path\to\file""",
                jsonElement!!.asJsonObject.getAsJsonObject("payload").get("reason").asString
            )
        }

    @Test
    fun `KR41-07b kickUser reason with both double-quote and backslash - WS message is valid JSON`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            val nastyReason = """say \"weird\" \thing"""
            viewModel.kickUser("target-user-3", nastyReason)
            advanceUntilIdle()

            val kickMsg = fakeWsClient.sentMessages.firstOrNull {
                it.contains(""""type":"KickUser"""")
            }
            assertFalse("Should send KickUser WS message", kickMsg == null)

            val jsonElement = try {
                com.google.gson.JsonParser.parseString(kickMsg!!)
            } catch (e: com.google.gson.JsonParseException) {
                null
            }
            assertTrue(
                "WS message must be valid JSON for complex escape input; got: $kickMsg",
                jsonElement != null && jsonElement.isJsonObject
            )
            assertEquals(
                "reason field must preserve the original text (under payload)",
                nastyReason,
                jsonElement!!.asJsonObject.getAsJsonObject("payload").get("reason").asString
            )
        }
    // ─── TC-WS-CONNECT-01: joinRoom 应在 sendEnvelope 之前调用 wsClient.connect ──

    @Test
    fun `TC-WS-CONNECT-01 joinRoom should call wsClient connect before sending JoinRoom envelope`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given
            val fakeWsClientLocal = FakeWebSocketClient()
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val wsUrl = "ws://test-host:3000/ws"
            val viewModelLocal = RoomViewModel(
                wsClient = fakeWsClientLocal,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = wsUrl,
            )

            // When
            viewModelLocal.joinRoom("room-001")
            advanceUntilIdle()

            // Then: connect should have been called with correct URL
            assertTrue(
                "wsClient.connect() should have been called at least once",
                fakeWsClientLocal.connectCallCount > 0
            )
            assertEquals(
                "connect URL should include token as query param",
                "ws://test-host:3000/ws?token=test-jwt-token",
                fakeWsClientLocal.lastConnectedUrl
            )
        }

    // ─── TC-WS-CONNECT-02: joinRoom 竞态保护 — 等待 Connected 后才发 JoinRoom ───
    //
    // 问题：wsClient.connect() 在真实 OkHttp 中是异步的（仅启动握手，不等待 onOpen）。
    // 若 sendEnvelope("JoinRoom") 在 WS 仍处于 Connecting 时调用，send() 返回 false，
    // 消息被静默丢弃。
    //
    // 修复：connect() 之后用 state.first { Connected || Error || Disconnected } 等待就绪。
    //
    // [RED]  当前代码无等待：runCurrent() 后 JoinRoom 已被 send()=false 丢弃，
    //        simulateConnect() 后 advanceUntilIdle() 也不会补发 → 第二个断言 FAIL。
    // [GREEN] 修复后：coroutine 在 state.first{} 处挂起；simulateConnect() 触发恢复；
    //         JoinRoom 在 Connected 之后正常发出 → 两个断言均 PASS。

    @Test
    fun `TC-WS-CONNECT-02 joinRoom should wait for WS Connected before sending JoinRoom envelope`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given: FakeWebSocketClient that stays in Connecting until simulateConnect() is called
            val delayedWsClient = FakeWebSocketClient(autoConnect = false)
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val viewModelLocal = RoomViewModel(
                wsClient = delayedWsClient,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = "ws://test-host:3000/ws",
            )

            // When: joinRoom is called; WS is still Connecting (autoConnect=false)
            viewModelLocal.joinRoom("room-001")
            runCurrent()  // Advance the launched coroutine until it suspends at state.first{}

            // Then (intermediate): JoinRoom should NOT have been sent yet
            // (WS still in Connecting state — send() drops the message if not Connected)
            assertTrue(
                "JoinRoom envelope must NOT be sent while WS is still Connecting",
                delayedWsClient.sentMessages.none { it.contains("JoinRoom") }
            )

            // When: the connection is established (simulates OkHttp onOpen callback)
            delayedWsClient.simulateConnect()
            advanceUntilIdle()  // Resume the suspended coroutine and run to completion

            // Then: JoinRoom should be sent now that WS is Connected
            assertTrue(
                "JoinRoom envelope MUST be sent after WS transitions to Connected",
                delayedWsClient.sentMessages.any { it.contains("JoinRoom") }
            )
        }

    // ─── TC-WS-CONNECT-03: 重复 joinRoom 幂等性 — 不应 double-connect ───────────
    //
    // 问题：每次 joinRoom() 都调用 wsClient.connect()，没有幂等保护。
    // 若 joinRoom 被重复调用（同房间），旧 socket 未关闭，新 socket 被创建。
    //
    // 修复：在 joinRoom() 入口添加守卫：若已 Connected 且 roomId 相同，直接返回。
    //
    // [RED]  当前代码无守卫：两次 joinRoom → connectCallCount=2 → 断言 FAIL。
    // [GREEN] 修复后：第二次 joinRoom 命中守卫直接返回 → connectCallCount=1 → PASS。

    @Test
    fun `TC-WS-CONNECT-03 joinRoom called twice with same roomId should not double-connect`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given
            val fakeWsClientLocal = FakeWebSocketClient()
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val viewModelLocal = RoomViewModel(
                wsClient = fakeWsClientLocal,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = "ws://test-host:3000/ws",
            )

            // When: joinRoom called twice with the same roomId
            viewModelLocal.joinRoom("room-001")
            advanceUntilIdle()  // First call fully completes; WS is now Connected

            viewModelLocal.joinRoom("room-001")
            advanceUntilIdle()  // Second call — should be a no-op (already Connected)

            // Then: connect() must only have been called once (idempotent)
            assertEquals(
                "wsClient.connect() should be called exactly once when joinRoom is called twice with same roomId",
                1,
                fakeWsClientLocal.connectCallCount
            )
        }

    // ─── TC-WS-CONNECT-04: Connecting 期间的幂等性 ────────────────────────────
    //
    // 问题：旧守卫只检查 Connected 状态，Connecting 期间重复调用 joinRoom 同房间时
    //       没有被拦截，导致 double-connect（第二个 connect() 调用被发出）。
    //
    // 修复：将守卫扩展到 Connecting / Message 状态，或使用 joinJob 追踪。
    //
    // [RED]  当前代码：第二次调用时 state=Connecting（非 Connected），守卫不拦截 →
    //        connectCallCount=2 → 断言 FAIL。
    // [GREEN] 修复后：守卫扩展包含 Connecting → 第二次调用直接返回 → connectCallCount=1。

    @Test
    fun `TC-WS-CONNECT-04 joinRoom called again while still connecting should not double-connect`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given: FakeWebSocketClient with autoConnect=false, connect() stays at Connecting
            val fakeWsClientLocal = FakeWebSocketClient(autoConnect = false)
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val viewModelLocal = RoomViewModel(
                wsClient = fakeWsClientLocal,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = "ws://test-host:3000/ws",
            )

            // When: first joinRoom starts (WS transitions to Connecting but does not complete)
            viewModelLocal.joinRoom("room-001")
            runCurrent()  // let the coroutine run until it suspends at state.first{}

            // Sanity check: state should be Connecting (not Connected)
            assertTrue(
                "WS state should be Connecting after first joinRoom with autoConnect=false",
                fakeWsClientLocal.state.value is WebSocketState.Connecting
            )

            // When: second joinRoom called while WS is still Connecting (same room)
            viewModelLocal.joinRoom("room-001")
            runCurrent()

            // Then: connect should only have been called once (idempotent guard covers Connecting)
            assertEquals(
                "connect() must be called exactly once even if joinRoom is repeated during Connecting",
                1,
                fakeWsClientLocal.connectCallCount
            )
        }

    // ─── TC-WS-CONNECT-05: 切换房间时应先 disconnect 旧连接 ───────────────────
    //
    // 问题：joinRoom("room-002") 之前没有调用 disconnect("room-001")，
    //       导致旧 socket listener 继续回调，污染新房间状态。
    //
    // 修复：在 joinRoom() 入口，若房间 ID 不同，先 cancel joinJob 并调用 disconnect()。
    //
    // [RED]  当前代码无 disconnect 调用 → disconnectCallCount=0 → 断言 FAIL。
    // [GREEN] 修复后：切换房间时先 disconnect → disconnectCallCount>0 → PASS。

    @Test
    fun `TC-WS-CONNECT-05 joinRoom with different roomId should disconnect previous connection`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given
            val fakeWsClientLocal = FakeWebSocketClient()
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val viewModelLocal = RoomViewModel(
                wsClient = fakeWsClientLocal,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = "ws://test-host:3000/ws",
            )

            // When: join room-001 first (completes fully → state=Connected)
            viewModelLocal.joinRoom("room-001")
            advanceUntilIdle()

            assertEquals(
                "connectCallCount should be 1 after joining room-001",
                1,
                fakeWsClientLocal.connectCallCount
            )

            // When: switch to room-002
            viewModelLocal.joinRoom("room-002")
            advanceUntilIdle()

            // Then: disconnect must have been called for the old room-001 connection
            assertTrue(
                "disconnect() must be called when switching from room-001 to room-002",
                fakeWsClientLocal.disconnectCallCount > 0
            )
            // And: connect should have been called twice (once per room)
            assertEquals(
                "connect() should be called exactly twice (room-001 then room-002)",
                2,
                fakeWsClientLocal.connectCallCount
            )
        }

    // ─── TC-WS-CONNECT-06: WS 连接超时失败后允许重试 ───────────────────────────
    //
    // 问题：joinRoom 失败路径不清理状态，导致重试被幂等保护拦截。
    // 当 WS 连接超时后，currentRoomId 仍为该房间，wsClient 状态仍为 Connecting，
    // 幂等检查 (joiningRoomId==roomId && Connecting) 会拦截同房间的重试调用。
    //
    // 修复：失败路径清理 joiningRoomId/currentRoomId 并调用 wsClient.disconnect()，
    // 使下次调用同房间 joinRoom 不再被幂等保护卡住。
    //
    // [RED]  当前代码：超时后 currentRoomId 仍为 "room-001"，state 仍 Connecting →
    //        重试被幂等保护拦截 → connect() 未被调用 → 断言 FAIL。
    // [GREEN] 修复后：失败路径清理状态 → 重试时幂等检查不命中 → connect() 再次被调用 → PASS。

    @Test
    fun `TC-WS-CONNECT-06 joinRoom should allow retry after WS connection timeout`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Given: FakeWebSocketClient that never auto-connects (stays in Connecting)
            val fakeWsClientLocal = FakeWebSocketClient(autoConnect = false)
            val fakeTokenManager = object : ITokenManager {
                override suspend fun getToken() = "test-jwt-token"
                override suspend fun saveToken(token: String) {}
                override suspend fun clearToken() {}
            }
            val viewModelLocal = RoomViewModel(
                wsClient = fakeWsClientLocal,
                roomSnapshotRepository = FakeRoomSnapshotRepository(defaultSnapshot),
                tokenManager = fakeTokenManager,
                wsUrl = "ws://test-host:3000/ws",
            )

            // When: first joinRoom starts and times out (ViewModel has 5s timeout)
            viewModelLocal.joinRoom("room-001")
            advanceTimeBy(6_000L)  // exceed 5-second WS connect timeout
            advanceUntilIdle()

            // Verify: first attempt resulted in Error state
            assertTrue(
                "uiState should be Error after WS connection timeout, was: ${viewModelLocal.uiState.value}",
                viewModelLocal.uiState.value is RoomViewState.Error
            )

            // Record connect call count before retry
            val connectCountBeforeRetry = fakeWsClientLocal.connectCallCount

            // When: retry the same room — must NOT be blocked by idempotency guard
            viewModelLocal.joinRoom("room-001")
            runCurrent()  // advance coroutine until it calls connect() and suspends

            // Then: connect() should have been called again (retry was not blocked)
            assertTrue(
                "connect() must be called again on retry. Before=$connectCountBeforeRetry, " +
                    "After=${fakeWsClientLocal.connectCallCount}",
                fakeWsClientLocal.connectCallCount > connectCountBeforeRetry
            )
        }
}

// ─── Test Doubles ─────────────────────────────────────────────────────────────

/**
 * [IRoomSnapshotRepository] 的测试 Fake 实现
 *
 * 默认返回构造传入的 [snapshot]。
 * 设置 [throwError] 后，下次调用 [getRoomSnapshot] 时抛出该异常（用于测试 VM-02）。
 */
class FakeRoomSnapshotRepository(
    private val snapshot: RoomSnapshot
) : IRoomSnapshotRepository {

    var throwError: Exception? = null

    override suspend fun getRoomSnapshot(roomId: String): RoomSnapshot {
        throwError?.let { throw it }
        return snapshot
    }
}
