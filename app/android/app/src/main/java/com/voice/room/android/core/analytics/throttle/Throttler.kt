package com.voice.room.android.core.analytics.throttle

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import com.voice.room.android.core.analytics.session.Clock
import com.voice.room.android.core.analytics.session.SystemClock

/**
 * Flush 触发器（T-30035）
 *
 * 决定何时将队列中的事件批量上报：
 * - 队列 ≥ [batchSize]（默认 8）条时立即 flush
 * - 距上次 flush ≥ [flushIntervalMs]（默认 2min）时 flush
 * - `onStop()` / `onWsReconnected()` 时立即 flush（由外部调用）
 *
 * @param batchSize       触发 flush 的队列阈值
 * @param flushIntervalMs 定时 flush 间隔（毫秒）
 * @param clock           时钟（测试注入 FakeClock）
 * @param scope           协程作用域（flush 在此 scope 中启动）
 * @param doFlush         flush 执行体（suspend 函数）
 */
class Throttler(
    private val batchSize: Int = 8,
    private val flushIntervalMs: Long = 2L * 60_000L,
    private val clock: Clock = SystemClock,
    private val scope: CoroutineScope,
    private val doFlush: suspend () -> Unit
) {
    @Volatile
    private var lastFlushAt: Long = 0L

    /**
     * 队列大小变化通知。
     * 若满足 batch 或时间条件，自动在 [scope] 中启动 flush。
     */
    fun notify(queueSize: Int) {
        val now = clock.now()
        val timeSinceLastFlush = now - lastFlushAt
        if (queueSize >= batchSize || timeSinceLastFlush >= flushIntervalMs) {
            lastFlushAt = now
            scope.launch { doFlush() }
        }
    }

    /** App 退后台时调用，立即 flush */
    fun onStop() {
        lastFlushAt = clock.now()
        scope.launch { doFlush() }
    }

    /** WS 重连成功时调用，立即 flush 缓存事件 */
    fun onWsReconnected() {
        lastFlushAt = clock.now()
        scope.launch { doFlush() }
    }
}
