package com.voice.room.android.feature.room

import com.voice.room.android.data.local.FakeKickCooldownStore
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — HallViewModel 进房前 kickCooldown 检查 (T-30042)
 *
 * K42-03: cooldown 未过期 → HallViewModel 发出 ShowToast 事件
 * K42-04: cooldown 已过期 → 正常进房（无 ShowToast）
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
            // 设置 cooldown 未过期：还剩 300 秒
            val futureMs = System.currentTimeMillis() + 300_000L
            store.save("room-42", futureMs)

            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store
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
            // Toast 消息应包含剩余秒数提示
            assertTrue(
                "Toast message should mention remaining seconds",
                toastEvents.first().message.contains("秒")
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
            // 设置 cooldown 已过期：过去的时间戳
            val pastMs = System.currentTimeMillis() - 1_000L
            store.save("room-42", pastMs)

            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store
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
            // 不设置任何 cooldown
            val viewModel = HallViewModel(
                roomRepository = FakeRoomRepository(),
                kickCooldownStore = store
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
}
