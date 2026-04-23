package com.voice.room.android.feature.room.governance

import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — MuteCountdownViewModel (T-30042)
 *
 * K42-05: startMicCountdown(expiresAt) → micExpiresAt 非 null
 * K42-06: micRemainingSeconds 正确计算（注入 FakeClock）
 * K42-07: duration_sec=0 时调用 clearMic → micExpiresAt=null
 * K42-08: 连续两次 startMicCountdown 取最新 expiresAt（直接覆盖）
 * K42-09: startMicCountdown 不影响 chatExpiresAt（mic/chat 独立）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MuteCountdownViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── K42-05: startMicCountdown → micExpiresAt 非 null ────────────────────

    @Test
    fun `K42-05 startMicCountdown sets micExpiresAt to non-null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            val expiresAt = 1_000_000L + 300_000L  // 5 分钟后
            vm.startMicCountdown(expiresAt)
            advanceUntilIdle()

            assertNotNull("micExpiresAt should be non-null after startMicCountdown", vm.micExpiresAt.value)
            assertEquals("micExpiresAt should equal the provided value", expiresAt, vm.micExpiresAt.value)
        }

    // ─── K42-06: micRemainingSeconds 正确计算 ────────────────────────────────

    @Test
    fun `K42-06 micRemainingSeconds returns correct remaining seconds based on clock`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val nowMs = 1_000_000L
            val fakeClock = FakeClock(currentTimeMs = nowMs)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            val expiresAt = nowMs + 120_000L  // 120 秒后到期
            vm.startMicCountdown(expiresAt)
            advanceUntilIdle()

            val remaining = vm.micRemainingSeconds()
            assertEquals("micRemainingSeconds should be 120", 120L, remaining)
        }

    @Test
    fun `K42-06b micRemainingSeconds returns 0 when already expired`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val nowMs = 1_000_000L
            val fakeClock = FakeClock(currentTimeMs = nowMs)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            val expiresAt = nowMs - 5_000L  // 已过期
            vm.startMicCountdown(expiresAt)
            advanceUntilIdle()

            val remaining = vm.micRemainingSeconds()
            assertEquals("micRemainingSeconds should be 0 when expired", 0L, remaining)
        }

    // ─── K42-07: duration_sec=0 → clearMic → micExpiresAt=null ──────────────

    @Test
    fun `K42-07 clearMic sets micExpiresAt to null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            vm.startMicCountdown(1_000_000L + 300_000L)
            advanceUntilIdle()
            assertNotNull("precondition: micExpiresAt should be set", vm.micExpiresAt.value)

            vm.clearMic()
            advanceUntilIdle()

            assertNull("micExpiresAt should be null after clearMic", vm.micExpiresAt.value)
        }

    @Test
    fun `K42-07b clearChat sets chatExpiresAt to null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            vm.startChatCountdown(1_000_000L + 300_000L)
            advanceUntilIdle()
            assertNotNull("precondition: chatExpiresAt should be set", vm.chatExpiresAt.value)

            vm.clearChat()
            advanceUntilIdle()

            assertNull("chatExpiresAt should be null after clearChat", vm.chatExpiresAt.value)
        }

    // ─── K42-08: 连续两次调用取最新 expiresAt ────────────────────────────────

    @Test
    fun `K42-08 consecutive startMicCountdown calls use latest expiresAt`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            val firstExpires = 1_000_000L + 100_000L   // 100 秒后
            val secondExpires = 1_000_000L + 300_000L  // 300 秒后（最新）

            vm.startMicCountdown(firstExpires)
            advanceUntilIdle()
            vm.startMicCountdown(secondExpires)
            advanceUntilIdle()

            assertEquals(
                "micExpiresAt should be the latest (second) value",
                secondExpires,
                vm.micExpiresAt.value
            )
            val remaining = vm.micRemainingSeconds()
            assertEquals("micRemainingSeconds should reflect latest expiresAt", 300L, remaining)
        }

    // ─── K42-09: mic/chat 独立，互不影响 ─────────────────────────────────────

    @Test
    fun `K42-09 startMicCountdown does not affect chatExpiresAt`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            vm.startMicCountdown(1_000_000L + 300_000L)
            advanceUntilIdle()

            assertNull("chatExpiresAt should remain null when only mic is muted", vm.chatExpiresAt.value)
        }

    @Test
    fun `K42-09b startChatCountdown does not affect micExpiresAt`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            vm.startChatCountdown(1_000_000L + 300_000L)
            advanceUntilIdle()

            assertNull("micExpiresAt should remain null when only chat is muted", vm.micExpiresAt.value)
        }

    @Test
    fun `K42-09c mic and chat countdowns coexist independently`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val nowMs = 1_000_000L
            val fakeClock = FakeClock(currentTimeMs = nowMs)
            val vm = MuteCountdownViewModel(clock = fakeClock)

            val micExpires = nowMs + 120_000L   // mic: 120 秒后
            val chatExpires = nowMs + 240_000L  // chat: 240 秒后

            vm.startMicCountdown(micExpires)
            vm.startChatCountdown(chatExpires)
            advanceUntilIdle()

            assertEquals("micExpiresAt should be set independently", micExpires, vm.micExpiresAt.value)
            assertEquals("chatExpiresAt should be set independently", chatExpires, vm.chatExpiresAt.value)

            assertEquals("micRemainingSeconds should be 120", 120L, vm.micRemainingSeconds())
            assertEquals("chatRemainingSeconds should be 240", 240L, vm.chatRemainingSeconds())

            // clearMic 不应影响 chatExpiresAt
            vm.clearMic()
            advanceUntilIdle()
            assertNull("micExpiresAt should be null after clearMic", vm.micExpiresAt.value)
            assertEquals("chatExpiresAt should remain after clearMic", chatExpires, vm.chatExpiresAt.value)
        }
}
