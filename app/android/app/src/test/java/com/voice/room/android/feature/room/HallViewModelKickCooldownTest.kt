package com.voice.room.android.feature.room

import com.voice.room.android.R
import com.voice.room.android.data.local.FakeKickCooldownStore
import com.voice.room.android.data.local.InMemoryKickCooldownStore
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.util.UiText
import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — HallViewModel 进房前 kickCooldown 检查 (T-30042)
 *
 * K42-03: cooldown 未过期 → HallViewModel 发出 ShowToast 事件
 * K42-04: cooldown 已过期 → 正常进房（无 ShowToast）
 * K42-C2: 共享 store — RoomViewModel.acknowledgeKick() 写入后 HallViewModel.enterRoom() 能检测到
 */
@OptIn(ExperimentalCoroutinesApi::class)
class HallViewModelKickCooldownTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── K42-03: cooldown 未过期 → Toast 拦截 ────────────────────────────────

    @Test
    fun `K42-03 enterRoom within cooldown period emits ShowToast and blocks navigation`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val store = FakeKickCooldownStore()
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            // 设置 cooldown 未过期：还剩 300 秒
            val futureMs = fakeClock.currentTimeMs + 300_000L
            store.save("room-42", futureMs)

            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store,
                clock = fakeClock,
            )
            advanceUntilIdle()

            val collectedEvents = mutableListOf<HallEvent>()
            val job = launch {
                viewModel.hallEvents.collect { collectedEvents.add(it) }
            }

            viewModel.enterRoom("room-42")
            advanceUntilIdle()

            // 应该有 ShowToast 事件
            val toastEvents = collectedEvents.filterIsInstance<HallEvent.ShowToast>()
            assertTrue(
                "Should emit ShowToast when cooldown active",
                toastEvents.isNotEmpty()
            )
            // Toast 消息应使用国际化资源（缺陷 #4）
            val text = toastEvents.first().text
            assertTrue("Toast text should be UiText.StringResource", text is UiText.StringResource)
            assertEquals(
                "Toast should reference R.string.hall_kick_cooldown_seconds",
                R.string.hall_kick_cooldown_seconds,
                (text as UiText.StringResource).resId,
            )
            assertTrue(
                "Toast args should contain remaining seconds",
                text.args.isNotEmpty()
            )
            // 不应该有 NavigateToRoom 事件
            assertFalse(
                "Should NOT emit NavigateToRoom when cooldown active",
                collectedEvents.any { it is HallEvent.NavigateToRoom }
            )

            job.cancel()
        }

    // ─── K42-04: cooldown 已过期 → 正常进房 ───────────────────────────────────

    @Test
    fun `K42-04 enterRoom after cooldown expired navigates normally without Toast`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val store = FakeKickCooldownStore()
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            // 设置 cooldown 已过期：过去的时间戳
            val pastMs = fakeClock.currentTimeMs - 1_000L
            store.save("room-42", pastMs)

            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store,
                clock = fakeClock,
            )
            advanceUntilIdle()

            val collectedEvents = mutableListOf<HallEvent>()
            val job = launch {
                viewModel.hallEvents.collect { collectedEvents.add(it) }
            }

            viewModel.enterRoom("room-42")
            advanceUntilIdle()

            // 不应该有 ShowToast 事件（cooldown 已过期）
            assertFalse(
                "Should NOT emit ShowToast when cooldown expired",
                collectedEvents.any { it is HallEvent.ShowToast }
            )
            // 应该有 NavigateToRoom 事件
            assertTrue(
                "Should emit NavigateToRoom when cooldown expired",
                collectedEvents.any { it is HallEvent.NavigateToRoom }
            )

            job.cancel()
        }

    // ─── 边界：无 cooldown 记录时正常进房 ──────────────────────────────────────

    @Test
    fun `K42-04b enterRoom with no cooldown record navigates normally`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val store = FakeKickCooldownStore()
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            // 不设置任何 cooldown
            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store,
                clock = fakeClock,
            )
            advanceUntilIdle()

            val collectedEvents = mutableListOf<HallEvent>()
            val job = launch {
                viewModel.hallEvents.collect { collectedEvents.add(it) }
            }

            viewModel.enterRoom("room-99")
            advanceUntilIdle()

            assertFalse(
                "Should NOT emit ShowToast when no cooldown record",
                collectedEvents.any { it is HallEvent.ShowToast }
            )
            assertTrue(
                "Should emit NavigateToRoom when no cooldown",
                collectedEvents.any { it is HallEvent.NavigateToRoom }
            )

            job.cancel()
        }

    // ─── K42-C2: 共享 store — RoomViewModel.acknowledgeKick 写入 → HallViewModel 能读到 ──

    @Test
    fun `K42-C2 shared store — acknowledgeKick in Room blocks re-entry in Hall`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 共享同一个 KickCooldownStore 实例（模拟 Application 单例行为）
            val sharedStore = InMemoryKickCooldownStore()
            val fixedNowMs = 2_000_000L
            val roomClock = FakeClock(currentTimeMs = fixedNowMs)
            val hallClock = FakeClock(currentTimeMs = fixedNowMs)

            val fakeWsClient = FakeWebSocketClient()
            val fakeRepo = FakeRoomSnapshotRepository(
                RoomSnapshot(
                    roomId = "room-42",
                    roomName = "T42",
                    onlineCount = 1,
                    micSlots = listOf(MicSlotData(0, "u1", "Nick"))
                )
            )

            val roomVm = RoomViewModel(
                wsClient = fakeWsClient,
                roomSnapshotRepository = fakeRepo,
                kickCooldownStore = sharedStore,
                clock = roomClock,
            )
            val hallVm = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = sharedStore,
                clock = hallClock,
            )

            // 1. 进入房间
            roomVm.joinRoom("room-42", "u1")
            advanceUntilIdle()

            // 2. 收到 UserKicked
            fakeWsClient.simulateMessage(
                """{"type":"UserKicked","reason":"spam","cooldown_sec":600}"""
            )
            advanceUntilIdle()

            // 3. 用户确认 → acknowledgeKick 写入 sharedStore（until = fixedNowMs + 600_000）
            roomVm.acknowledgeKick()
            advanceUntilIdle()

            // 验证 store 中有写入
            val savedUntil = sharedStore.get("room-42")
            assertEquals(
                "sharedStore should contain cooldown after acknowledgeKick",
                fixedNowMs + 600_000L,
                savedUntil
            )

            // 4. hallClock 仍在 fixedNowMs（cooldown 未过期），enterRoom 应被拦截
            val hallEvents = mutableListOf<HallEvent>()
            val job = launch {
                hallVm.hallEvents.collect { hallEvents.add(it) }
            }

            hallVm.enterRoom("room-42")
            advanceUntilIdle()

            assertTrue(
                "HallViewModel should block entry with ShowToast when cooldown active",
                hallEvents.any { it is HallEvent.ShowToast }
            )
            assertFalse(
                "HallViewModel should NOT navigate to room when cooldown active",
                hallEvents.any { it is HallEvent.NavigateToRoom }
            )

            // 5. 时间推进到 cooldown 之后
            hallClock.currentTimeMs = fixedNowMs + 601_000L
            hallEvents.clear()

            hallVm.enterRoom("room-42")
            advanceUntilIdle()

            assertFalse(
                "HallViewModel should NOT show toast after cooldown expires",
                hallEvents.any { it is HallEvent.ShowToast }
            )
            assertTrue(
                "HallViewModel should navigate after cooldown expires",
                hallEvents.any { it is HallEvent.NavigateToRoom }
            )

            job.cancel()
        }
}
