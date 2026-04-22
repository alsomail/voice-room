package com.voice.room.android.core.analytics.eventreport

import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.queue.InMemoryEventQueueDao
import com.voice.room.android.core.analytics.session.Clock
import com.voice.room.android.core.analytics.session.SessionManager
import com.voice.room.android.core.analytics.throttle.Throttler
import com.voice.room.android.core.analytics.transport.SendOutcome
import com.voice.room.android.core.analytics.transport.Transport
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestCoroutineScheduler
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

/**
 * Throttler 和 SessionManager 补充测试（T-30035）
 *
 * 提高覆盖率，测试 onStop / onWsReconnected / session 边界。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ThrottlerSessionTest {

    private class FakeClock(var timeMs: Long = 0L) : Clock {
        override fun now(): Long = timeMs
    }

    private class RecordingTransport : Transport {
        var callCount = 0
        override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
            callCount++
            return Result.success(SendOutcome(batch.map { it.id }))
        }
    }

    private lateinit var scheduler: TestCoroutineScheduler
    private lateinit var testScope: TestScope
    private lateinit var fakeClock: FakeClock
    private lateinit var queueDao: InMemoryEventQueueDao
    private var flushCallCount = 0

    @Before
    fun setUp() {
        scheduler = TestCoroutineScheduler()
        testScope = TestScope(StandardTestDispatcher(scheduler))
        fakeClock = FakeClock(0L)
        queueDao = InMemoryEventQueueDao()
        flushCallCount = 0
    }

    private fun makeThrottler(
        batchSize: Int = 8,
        intervalMs: Long = 2L * 60_000L
    ): Throttler = Throttler(
        batchSize = batchSize,
        flushIntervalMs = intervalMs,
        clock = fakeClock,
        scope = testScope
    ) { flushCallCount++ }

    // ── Throttler: onStop 立即触发 flush ─────────────────────────────────

    @Test
    fun `Throttler onStop triggers flush immediately`() = testScope.runTest {
        val throttler = makeThrottler()

        throttler.onStop()
        advanceUntilIdle()

        assertEquals("onStop 应触发 1 次 flush", 1, flushCallCount)
    }

    // ── Throttler: onWsReconnected 立即触发 flush ─────────────────────────

    @Test
    fun `Throttler onWsReconnected triggers flush immediately`() = testScope.runTest {
        val throttler = makeThrottler()

        throttler.onWsReconnected()
        advanceUntilIdle()

        assertEquals("WS 重连应触发 1 次 flush", 1, flushCallCount)
    }

    // ── Throttler: 小于 batchSize 不触发 flush ───────────────────────────

    @Test
    fun `Throttler does not flush if below batch size and within time`() = testScope.runTest {
        val throttler = makeThrottler(batchSize = 8, intervalMs = 120_000L)

        // 只通知 7 次，且时间不够
        fakeClock.timeMs = 0L
        repeat(7) { throttler.notify(it + 1) }
        advanceUntilIdle()

        assertEquals("不足 8 条且时间未到，不应 flush", 0, flushCallCount)
    }

    // ── Throttler: 恰好 batchSize 触发 flush ─────────────────────────────

    @Test
    fun `Throttler flushes exactly at batch size`() = testScope.runTest {
        val throttler = makeThrottler(batchSize = 8)

        throttler.notify(8)
        advanceUntilIdle()

        assertEquals("恰好 8 条时应触发 flush", 1, flushCallCount)
    }

    // ── SessionManager: 多次前台不重置（未超时）───────────────────────────

    @Test
    fun `SessionManager keeps same id across multiple short foreground trips`() {
        fakeClock.timeMs = 0L
        val sm = SessionManager(clock = fakeClock, sessionTimeoutMs = 30_000L)
        sm.onForeground()
        val id1 = sm.currentId

        fakeClock.timeMs = 10_000L
        sm.onBackground()
        fakeClock.timeMs = 20_000L
        sm.onForeground()
        val id2 = sm.currentId

        assertEquals("短暂后台 session 不应改变", id1, id2)
    }

    // ── SessionManager: 恰好 30s 应生成新 session ───────────────────────

    @Test
    fun `SessionManager creates new session at exactly 30s boundary`() {
        fakeClock.timeMs = 0L
        val sm = SessionManager(clock = fakeClock, sessionTimeoutMs = 30_000L)
        sm.onForeground()
        val oldId = sm.currentId

        sm.onBackground()
        fakeClock.timeMs = 30_000L  // 恰好 30s
        sm.onForeground()

        assertNotEquals("恰好 30s 应生成新 session", oldId, sm.currentId)
    }

    // ── SessionManager: 未进后台时 currentId 稳定 ─────────────────────────

    @Test
    fun `SessionManager currentId is stable without background`() {
        val sm = SessionManager(clock = fakeClock)
        val id1 = sm.currentId
        val id2 = sm.currentId
        assertEquals("未进后台 currentId 应稳定", id1, id2)
    }

    // ── SessionManager: onForeground 在未 onBackground 时不改变 ───────────

    @Test
    fun `SessionManager onForeground without prior background keeps same session`() {
        fakeClock.timeMs = 0L
        val sm = SessionManager(clock = fakeClock)
        sm.onForeground()
        val id1 = sm.currentId

        fakeClock.timeMs = 60_000L
        sm.onForeground()  // 没有先调用 onBackground
        val id2 = sm.currentId

        assertEquals("未 onBackground 时，onForeground 不改变 session", id1, id2)
    }
}
