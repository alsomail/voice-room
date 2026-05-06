package com.voice.room.android.feature.room

import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * RoomViewModelWsTest — WS sealed class 反序列化后 ViewModel 行为回归测试
 *
 * 验证 handleWsMessage 迁移至 WsServerMessage sealed class 后：
 * - 使用协议规范 JSON（payload-nested snake_case）正确更新 UI 状态
 * - Pong 消息不触发 Unknown 分支
 * - 未知信令不抛异常（Unknown 分支处理）
 * - 无 ?: return 静默吞错（需通过 REGRESSION-4 断言 Unknown 日志/分支被命中）
 *
 * PROTO-BINDING: doc/protocol/schemas/ws/
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomViewModelWsTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var viewModel: RoomViewModel

    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-ws-test",
        roomName = "WS Test Room",
        onlineCount = 3,
        micSlots = listOf(
            MicSlotData(index = 0, userId = "user-slot0", nickname = "Slot0User"),
            MicSlotData(index = 1, userId = null, nickname = null),
            MicSlotData(index = 2, userId = null, nickname = null),
        )
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        viewModel = RoomViewModel(fakeWsClient, fakeRepo)
    }

    // ─── REGRESSION-1: MicTaken 新协议格式 → 正确更新 micSlots ──────────────

    /**
     * REGRESSION-1: 收到符合协议规范的 MicTaken（payload.mic_index + payload.user_id）
     * → handleWsMessage 应正确将 slot-1 的 userId/nickname 更新。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
     */
    @Test
    fun `REGRESSION-1 MicTaken protocol format updates micSlots correctly`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            // 协议规范格式：payload 嵌套，snake_case 字段
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":1,"user_id":"new-user-1","nickname":"NewNick1","avatar":null},"msg_id":"msg-001","timestamp":1234567890}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            val slot1 = state.uiState.micSlots[1]
            assertEquals("slot-1 userId should be new-user-1", "new-user-1", slot1.userId)
            assertEquals("slot-1 nickname should be NewNick1", "NewNick1", slot1.nickname)
        }

    // ─── REGRESSION-2: UserJoined 新协议格式 → 正确更新 onlineCount ───────────

    /**
     * REGRESSION-2: 收到符合协议规范的 UserJoined（payload.user_id + payload.nickname）
     * → handleWsMessage 应将 onlineCount + 1。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json
     */
    @Test
    fun `REGRESSION-2 UserJoined protocol format increments onlineCount`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            val before = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","payload":{"user_id":"user-new","nickname":"NewUser","avatar":null,"member_count":4},"msg_id":"msg-002","timestamp":1234567891}"""
            )
            advanceUntilIdle()

            val after = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount
            assertEquals("onlineCount should increase by 1", before + 1, after)
        }

    // ─── REGRESSION-3: Pong 不触发 Unknown 分支 ──────────────────────────────

    /**
     * REGRESSION-3: Pong 消息应被解析为 WsServerMessage.Pong（不是 Unknown），
     * 且不应导致 UI 状态异常（onlineCount 不变）。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json
     */
    @Test
    fun `REGRESSION-3 Pong message processed without state corruption`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            val before = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount

            fakeWsClient.simulateMessage(
                """{"type":"Pong","msg_id":"ping-echo-001","timestamp":1234567892}"""
            )
            advanceUntilIdle()

            // Pong 不影响 onlineCount
            val after = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount
            assertEquals("Pong should not change onlineCount", before, after)
        }

    // ─── REGRESSION-4: 未知信令不崩溃 ───────────────────────────────────────

    /**
     * REGRESSION-4: 收到完全未知的信令类型时不抛出异常，UI 状态保持不变。
     * (Unknown 分支仅 log，不 crash)
     */
    @Test
    fun `REGRESSION-4 unknown signal type does not crash and state unchanged`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            val before = viewModel.uiState.value

            // 完全未知信令
            fakeWsClient.simulateMessage("""{"type":"FutureSuperSignal","payload":{"some":"data"}}""")
            advanceUntilIdle()

            // 状态不应改变
            assertEquals("Unknown signal should not change uiState", before, viewModel.uiState.value)
        }

    // ─── REGRESSION-5: UserJoined 新格式 → audience 追加 ────────────────────

    /**
     * REGRESSION-5: UserJoined（新协议格式）后，audienceState.audience 中应包含该用户。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json
     */
    @Test
    fun `REGRESSION-5 UserJoined protocol format adds user to audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","payload":{"user_id":"user-joined-5","nickname":"User5","avatar":null},"msg_id":"msg-005","timestamp":1234567895}"""
            )
            advanceUntilIdle()

            val aud = viewModel.audienceState.value
            assertTrue(
                "user-joined-5 should be in audience after UserJoined",
                aud.audience.any { it.id == "user-joined-5" }
            )
            assertEquals(
                "audience member nickname should be User5",
                "User5",
                aud.audience.find { it.id == "user-joined-5" }?.nickname
            )
        }

    // ─── REGRESSION-6: MicLeft 新协议格式 → slot 清空 ────────────────────────

    /**
     * REGRESSION-6: 收到符合协议规范的 MicLeft（payload.mic_index）
     * → handleWsMessage 应将 slot-0 的 userId 清空。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/MicLeft.schema.json
     */
    @Test
    fun `REGRESSION-6 MicLeft protocol format clears slot userId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            // slot-0 was occupied by "user-slot0" from snapshot
            fakeWsClient.simulateMessage(
                """{"type":"MicLeft","payload":{"mic_index":0,"user_id":"user-slot0"},"msg_id":"msg-006","timestamp":1234567896}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            val slot0 = state.uiState.micSlots[0]
            assertFalse(
                "slot-0 userId should be null after MicLeft",
                slot0.userId != null
            )
        }

    // ─── REGRESSION-7: UserLeft 新协议格式 → onlineCount-- ─────────────────

    /**
     * REGRESSION-7: 收到符合协议规范的 UserLeft（payload.user_id）
     * → handleWsMessage 应将 onlineCount - 1。
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json
     */
    @Test
    fun `REGRESSION-7 UserLeft protocol format decrements onlineCount`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            val before = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount

            fakeWsClient.simulateMessage(
                """{"type":"UserLeft","payload":{"user_id":"user-slot0"},"msg_id":"msg-007","timestamp":1234567897}"""
            )
            advanceUntilIdle()

            val after = (viewModel.uiState.value as RoomViewState.Success).uiState.onlineCount
            assertEquals("onlineCount should decrease by 1", before - 1, after)
        }

    // ─── REGRESSION-8: lastReceivedMsgId 从 msg_id 字段更新 ─────────────────

    /**
     * REGRESSION-8: 任何带 msg_id 的入站消息应更新 lastReceivedMsgId，
     * 用于断线重连重放机制。
     */
    @Test
    fun `REGRESSION-8 inbound msg_id updates lastReceivedMsgId for reconnect`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","payload":{"user_id":"u1","nickname":"N1","avatar":null},"msg_id":"reconnect-cursor-001","timestamp":123}"""
            )
            advanceUntilIdle()

            assertEquals(
                "lastReceivedMsgId should be updated from msg_id field",
                "reconnect-cursor-001",
                viewModel.lastReceivedMsgIdForTest()
            )
        }

    // ─── REGRESSION-9: MicTaken 自己上麦（新格式）→ isCurrentUserOnMic=true ──

    /**
     * REGRESSION-9: MicTaken with currentUser's userId (new protocol format)
     * → isCurrentUserOnMic should become true.
     *
     * PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
     */
    @Test
    fun `REGRESSION-9 MicTaken for self new format sets isCurrentUserOnMic true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-ws-test", userId = "self-user")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","payload":{"mic_index":2,"user_id":"self-user","nickname":"SelfNick","avatar":null},"msg_id":"msg-009","timestamp":123}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertTrue(
                "isCurrentUserOnMic should be true after MicTaken for self",
                state.uiState.isCurrentUserOnMic
            )
        }
}
