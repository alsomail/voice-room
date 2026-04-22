package com.voice.room.android.core.analytics.session

import java.util.UUID

/**
 * 时钟抽象（T-30035）
 *
 * 允许在测试中注入受控时钟，实现时间推进无需 `Thread.sleep()`。
 */
fun interface Clock {
    fun now(): Long
}

/** 系统真实时钟 */
object SystemClock : Clock {
    override fun now(): Long = System.currentTimeMillis()
}

/**
 * Session ID 管理器（T-30035）
 *
 * 规则：
 * - 首次进前台生成 UUID
 * - App 退后台 ≥30s 再回前台 → 生成新 UUID（新 session）
 * - `onForeground()` / `onBackground()` 由 App Lifecycle 回调驱动
 *
 * @param clock 可注入的时钟（测试时传入 FakeClock）
 * @param sessionTimeoutMs session 超时阈值，默认 30000ms
 */
class SessionManager(
    private val clock: Clock = SystemClock,
    private val sessionTimeoutMs: Long = 30_000L
) {
    @Volatile
    private var _sessionId: String = UUID.randomUUID().toString()

    @Volatile
    private var backgroundAt: Long? = null

    /** 当前 session UUID */
    val currentId: String get() = _sessionId

    /**
     * App 进入前台时调用。
     * 若后台时长 ≥ [sessionTimeoutMs] 则生成新 session_id。
     */
    fun onForeground() {
        val bgAt = backgroundAt
        if (bgAt != null && clock.now() - bgAt >= sessionTimeoutMs) {
            _sessionId = UUID.randomUUID().toString()
        }
        backgroundAt = null
    }

    /** App 进入后台时调用，记录时间 */
    fun onBackground() {
        backgroundAt = clock.now()
    }

    /** 强制刷新 session（仅测试用） */
    internal fun resetForTest() {
        _sessionId = UUID.randomUUID().toString()
        backgroundAt = null
    }
}
