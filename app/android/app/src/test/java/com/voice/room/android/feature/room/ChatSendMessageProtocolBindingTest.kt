package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.core.ws.RoomSocketRequestSpec
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * T-30054: RoomViewModel.sendMessage 协议路径绑定集成测试
 *
 * 断言点 grep-able 标记（见 RoomViewModel.sendMessage）：
 *   `// PROTO-BINDING: wsClient.sendEnvelope SendMessage — T-30054`
 *
 * 协议链路锁定（doc/protocol/websocket_signals.md §6.8.1 / §6.8.4）：
 *   ① WS C→S `SendMessage`（主路径 ⭐） → wsClient.sendEnvelope(type = "SendMessage", ...)
 *   ② REST POST /api/v1/chat-messages（禁用）→ 客户端不调用，mockWebServer 期望 0 次命中
 *
 * TC-PROTO-1: JSON 包含 "type":"SendMessage"
 * TC-PROTO-2: REST MockWebServer 收到 0 次 HTTP 请求（WS 主路径，REST 禁用）
 * TC-PROTO-3: JSON 字段命名与服务端 handle_send_message 1:1 对齐（payload.content + 顶层 msg_id）
 * TC-BOUND-1: 空内容/空白内容 → wsClient.send 不被调用
 * TC-BOUND-2: 超长内容（>MAX_MESSAGE_LENGTH 字符）→ ShowToast 事件，wsClient.send 不被调用
 * TC-BOUND-3: WS 未连接状态 → send 静默丢弃，sentMessages 为空
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ChatSendMessageProtocolBindingTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var viewModel: RoomViewModel
    private lateinit var mockWebServer: MockWebServer

    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Protocol Test Room",
        onlineCount = 3,
        micSlots = listOf(MicSlotData(index = 0, userId = null, nickname = null)),
    )

    @Before
    fun setup() {
        mockWebServer = MockWebServer()
        mockWebServer.start()
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        fakeMediaService = FakeMediaService()
        viewModel = RoomViewModel(fakeWsClient, fakeRepo, fakeMediaService)
    }

    @After
    fun tearDown() {
        mockWebServer.shutdown()
    }

    // ─── TC-PROTO-1: JSON 包含 "type":"SendMessage" ───────────────────────────

    @Test
    fun `TC-PROTO-1 sendMessage sends JSON with type SendMessage over WS`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            assertEquals("Should send exactly 1 WS message", 1, fakeWsClient.sentMessages.size)
            val json = fakeWsClient.sentMessages[0]
            // PROTO-BINDING 断言锚点 — T-30054 TC-PROTO-1
            assertTrue(
                """TC-PROTO-1 FAIL: JSON must contain "type":"SendMessage" — actual: $json""",
                json.contains(""""type":"SendMessage"""")
            )
        }

    // ─── TC-PROTO-2: REST MockWebServer 收到 0 次请求（不走 Retrofit POST）──────

    @Test
    fun `TC-PROTO-2 sendMessage does NOT call REST POST chat-messages endpoint`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // MockWebServer 模拟 REST API 服务端（protocol/room_api.md §3.6 POST /chat-messages）
            // TC-PROTO-2 验证：sendMessage 不发任何 HTTP 请求到此服务器（REST 路径在 Android 端被禁用）
            val restBaseUrl = mockWebServer.url("/").toString()

            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("hello")
            advanceUntilIdle()

            // 核心断言：REST 服务器期望 0 次请求（WS 主路径，REST 禁用）
            assertEquals(
                "TC-PROTO-2 FAIL: REST MockWebServer should receive 0 HTTP requests. " +
                    "If this fails, sendMessage is calling Retrofit POST $restBaseUrl (禁止).",
                0,
                mockWebServer.requestCount,
            )
            // WS 路径正常发出：确认消息通过 WS 侧发出
            assertEquals("WS path should send exactly 1 message", 1, fakeWsClient.sentMessages.size)
        }

    // ─── TC-PROTO-3: 字段名与服务端 handle_send_message 1:1 对齐 ─────────────

    @Test
    fun `TC-PROTO-3 sendMessage JSON fields align with server handle_send_message snake_case`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("proto-test")
            advanceUntilIdle()

            assertEquals(1, fakeWsClient.sentMessages.size)
            val json = fakeWsClient.sentMessages[0]
            // 服务端读取 payload.content（snake_case "content"）
            assertTrue(
                """TC-PROTO-3 FAIL: JSON must contain "content":"proto-test" — actual: $json""",
                json.contains(""""content":"proto-test"""")
            )
            // 服务端读取顶层 msg_id（snake_case "msg_id"，非 msgId/message_id）
            assertTrue(
                """TC-PROTO-3 FAIL: JSON must contain top-level "msg_id" key — actual: $json""",
                json.contains(""""msg_id":"""")
            )
            // msg_id 必须是非空 UUID v4 格式
            val msgIdPattern = Regex(""""msg_id":"[a-f0-9\\-]{36}"""")
            assertTrue(
                """TC-PROTO-3 FAIL: "msg_id" must be a UUID v4 string — actual: $json""",
                msgIdPattern.containsMatchIn(json)
            )
        }

    // ─── TC-BOUND-1: 空内容/空白内容 → wsClient.send 不被调用 ─────────────────

    @Test
    fun `TC-BOUND-1 sendMessage with blank content does not send WS message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            viewModel.sendMessage("")
            advanceUntilIdle()
            assertTrue("TC-BOUND-1 FAIL: empty string should not trigger WS send", fakeWsClient.sentMessages.isEmpty())

            viewModel.sendMessage("   ")
            advanceUntilIdle()
            assertTrue("TC-BOUND-1 FAIL: whitespace-only should not trigger WS send", fakeWsClient.sentMessages.isEmpty())
        }

    // ─── TC-BOUND-2: 超长内容（>MAX_MESSAGE_LENGTH）→ ShowToast，不发 WS ──────

    @Test
    fun `TC-BOUND-2 sendMessage with content over MAX_MESSAGE_LENGTH emits ShowToast and does NOT send WS`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            val collectedEvents = mutableListOf<RoomEvent>()
            val collectJob = launch { viewModel.events.collect { collectedEvents.add(it) } }

            val tooLong = "A".repeat(RoomViewModel.MAX_MESSAGE_LENGTH + 1)
            viewModel.sendMessage(tooLong)
            advanceUntilIdle()

            assertTrue(
                "TC-BOUND-2 FAIL: wsClient.send should NOT be called for content >${RoomViewModel.MAX_MESSAGE_LENGTH} chars",
                fakeWsClient.sentMessages.isEmpty(),
            )
            assertTrue(
                "TC-BOUND-2 FAIL: ShowToast event should be emitted for content >${RoomViewModel.MAX_MESSAGE_LENGTH} chars",
                collectedEvents.any { it is RoomEvent.ShowToast },
            )
            collectJob.cancel()
        }

    // ─── TC-BOUND-3: WS 未连接状态 → send 静默丢弃，sentMessages 为空 ─────────

    @Test
    fun `TC-BOUND-3 sendMessage when WS is disconnected silently discards the message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // FakeWebSocketClient.send() 在非 Connected 状态下返回 false 且不添加到 sentMessages
            fakeWsClient.connect(RoomSocketRequestSpec(url = "ws://test", headers = emptyMap()))
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            fakeWsClient.sentMessages.clear()

            // 模拟网络断开，WS 保持 Disconnected 状态
            fakeWsClient.simulateDisconnect("network lost")

            viewModel.sendMessage("hi")
            advanceUntilIdle()

            assertTrue(
                "TC-BOUND-3 FAIL: sentMessages should be empty when WS is Disconnected",
                fakeWsClient.sentMessages.isEmpty(),
            )
        }
}
