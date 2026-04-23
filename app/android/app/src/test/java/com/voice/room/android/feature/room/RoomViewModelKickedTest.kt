package com.voice.room.android.feature.room

import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.local.FakeKickCooldownStore
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — RoomViewModel 被踢提示弹窗 (T-30042)
 *
 * K42-01: 收到 UserKicked WS 消息 → kickedState 非 null（reason、cooldownSec 正确）
 * K42-02: acknowledgeKick() → 保存 cooldown 到 store + 发出 NavigateBack 事件
 * K42-C1: acknowledgeKick() 使用注入的 Clock（而非 System.currentTimeMillis()）计算 cooldown
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomViewModelKickedTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeStore: FakeKickCooldownStore
    private lateinit var fakeClock: FakeClock
    private lateinit var viewModel: RoomViewModel

    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-42",
        roomName = "Test Room",
        onlineCount = 3,
        micSlots = listOf(MicSlotData(index = 0, userId = "user-1", nickname = "Nick1"))
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        fakeStore = FakeKickCooldownStore()
        fakeClock = FakeClock(currentTimeMs = 1_000_000L)
        viewModel = RoomViewModel(
            wsClient = fakeWsClient,
            roomSnapshotRepository = fakeRepo,
            kickCooldownStore = fakeStore,
            clock = fakeClock,
        )
    }

    // ─── K42-01: UserKicked → kickedState 非 null ─────────────────────────────

    @Test
    fun `K42-01 UserKicked WS message sets kickedState with reason and cooldownSec`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 先进入房间，等 Success 状态
            viewModel.joinRoom("room-42", "user-1")
            advanceUntilIdle()

            // 收到 UserKicked WS 消息
            fakeWsClient.simulateMessage(
                """{"type":"UserKicked","reason":"spam","cooldown_sec":600}"""
            )
            advanceUntilIdle()

            val kicked = viewModel.kickedState.value
            assertNotNull("kickedState should be non-null after UserKicked", kicked)
            assertEquals("reason should be 'spam'", "spam", kicked!!.reason)
            assertEquals(
                "cooldownSec should be 600", 600, kicked.cooldownSec
            )
        }

    // ─── K42-02: acknowledgeKick() → 保存 cooldown + NavigateBack 事件 ─────────

    @Test
    fun `K42-02 acknowledgeKick saves cooldown to store and emits NavigateBack`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-42", "user-1")
            advanceUntilIdle()

            // 收集 events
            val collectedEvents = mutableListOf<RoomEvent>()
            val job = launch {
                viewModel.events.collect { collectedEvents.add(it) }
            }

            // 触发被踢
            fakeWsClient.simulateMessage(
                """{"type":"UserKicked","reason":"abuse","cooldown_sec":600}"""
            )
            advanceUntilIdle()

            // 用户确认知道了
            viewModel.acknowledgeKick()
            advanceUntilIdle()

            // 验证：cooldown 已写入 store（until > fakeClock.now + 599s）
            val untilMs = fakeStore.get("room-42")
            assertTrue(
                "cooldown untilMs should be in the future (> fakeClock.now + 599s)",
                untilMs > fakeClock.currentTimeMs + 599_000L
            )

            // 验证：发出 NavigateBack 事件
            assertTrue(
                "events should contain NavigateBack",
                collectedEvents.any { it is RoomEvent.NavigateBack }
            )

            // 验证：kickedState 已清空
            assertNull("kickedState should be null after acknowledgeKick", viewModel.kickedState.value)

            job.cancel()
        }

    // ─── K42-C1: acknowledgeKick 使用注入的 Clock 计算 cooldown 截止时间 ────────

    @Test
    fun `K42-C1 acknowledgeKick uses injected clock to calculate cooldown untilMs`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // fakeClock 固定在 1_000_000L ms
            fakeClock.currentTimeMs = 5_000_000L

            viewModel.joinRoom("room-42", "user-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserKicked","reason":"spam","cooldown_sec":600}"""
            )
            advanceUntilIdle()

            viewModel.acknowledgeKick()
            advanceUntilIdle()

            val untilMs = fakeStore.get("room-42")
            // 应该精确等于 fakeClock.currentTimeMs + 600_000L
            assertEquals(
                "untilMs should be fakeClock.now + 600s (using injected clock)",
                5_000_000L + 600_000L,
                untilMs
            )
        }
}
