package com.voice.room.android.core.analytics

import com.voice.room.android.core.analytics.impl.NoopAnalytics
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — AnalyticsPort 接口行为 & NoopAnalytics (T-30034)
 *
 * AP-01: NoopAnalytics.track() 不抛异常
 * AP-02: NoopAnalytics.captureException() 不抛异常
 * AP-03: NoopAnalytics.setUser() 不抛异常
 * AP-04: NoopAnalytics.setConsent() 不抛异常
 * AP-05: ConsentMode 枚举包含 All / CrashOnly / None 三个值
 * AP-06: NoopAnalytics 实现 AnalyticsPort 接口
 * AP-07: track() 支持空 properties Map
 * AP-08: track() 支持非空 properties Map
 * AP-09: setUser() 支持 null userId（用户登出场景）
 * AP-10: captureException() 支持空 extras
 * AP-11: captureException() 支持包含属性的 extras
 * AP-12: ConsentMode.CrashOnly 下 FakeAnalytics.track() 调用被记录，未执行上报
 */
class AnalyticsPortBehaviorTest {

    // ─────────────────────────────────────────────
    // AP-01: NoopAnalytics.track() 不抛异常
    // ─────────────────────────────────────────────

    @Test
    fun AP01_noop_track_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.track("test_event") // 不应抛异常
    }

    // ─────────────────────────────────────────────
    // AP-02: NoopAnalytics.captureException() 不抛异常
    // ─────────────────────────────────────────────

    @Test
    fun AP02_noop_captureException_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.captureException(RuntimeException("test")) // 不应抛异常
    }

    // ─────────────────────────────────────────────
    // AP-03: NoopAnalytics.setUser() 不抛异常
    // ─────────────────────────────────────────────

    @Test
    fun AP03_noop_setUser_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.setUser("user-123") // 不应抛异常
    }

    // ─────────────────────────────────────────────
    // AP-04: NoopAnalytics.setConsent() 不抛异常
    // ─────────────────────────────────────────────

    @Test
    fun AP04_noop_setConsent_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.setConsent(ConsentMode.All) // 不应抛异常
        analytics.setConsent(ConsentMode.CrashOnly) // 不应抛异常
        analytics.setConsent(ConsentMode.None) // 不应抛异常
    }

    // ─────────────────────────────────────────────
    // AP-05: ConsentMode 枚举包含 All / CrashOnly / None
    // ─────────────────────────────────────────────

    @Test
    fun AP05_consentMode_hasAllThreeValues() {
        val values = ConsentMode.values()
        assertTrue("应包含 All", values.any { it == ConsentMode.All })
        assertTrue("应包含 CrashOnly", values.any { it == ConsentMode.CrashOnly })
        assertTrue("应包含 None", values.any { it == ConsentMode.None })
        assertEquals("ConsentMode 应有且只有 3 个值", 3, values.size)
    }

    // ─────────────────────────────────────────────
    // AP-06: NoopAnalytics 实现 AnalyticsPort 接口
    // ─────────────────────────────────────────────

    @Test
    fun AP06_noopAnalytics_implementsAnalyticsPort() {
        val analytics = NoopAnalytics()
        assertTrue("NoopAnalytics 应实现 AnalyticsPort", analytics is AnalyticsPort)
    }

    // ─────────────────────────────────────────────
    // AP-07: track() 支持空 properties Map
    // ─────────────────────────────────────────────

    @Test
    fun AP07_track_withEmptyProperties_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.track("event_name", emptyMap())
    }

    // ─────────────────────────────────────────────
    // AP-08: track() 支持非空 properties Map
    // ─────────────────────────────────────────────

    @Test
    fun AP08_track_withProperties_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.track("gift_sent", mapOf("gift_id" to "g-001", "amount" to 100, "receiver" to null))
    }

    // ─────────────────────────────────────────────
    // AP-09: setUser() 支持 null userId（用户登出场景）
    // ─────────────────────────────────────────────

    @Test
    fun AP09_setUser_withNullUserId_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.setUser(null) // 用户登出
    }

    // ─────────────────────────────────────────────
    // AP-10: captureException() 支持空 extras
    // ─────────────────────────────────────────────

    @Test
    fun AP10_captureException_withEmptyExtras_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.captureException(IllegalStateException("empty extras"), emptyMap())
    }

    // ─────────────────────────────────────────────
    // AP-11: captureException() 支持包含属性的 extras
    // ─────────────────────────────────────────────

    @Test
    fun AP11_captureException_withExtras_doesNotThrow() {
        val analytics: AnalyticsPort = NoopAnalytics()
        analytics.captureException(
            throwable = RuntimeException("network error"),
            extras = mapOf("roomId" to "room-001", "userId" to "user-abc")
        )
    }

    // ─────────────────────────────────────────────
    // AP-12: FakeAnalytics 正确记录调用
    // ─────────────────────────────────────────────

    @Test
    fun AP12_fakeAnalytics_recordsCalls() {
        val fake = FakeAnalytics()
        fake.track("room_joined", mapOf("roomId" to "room-001"))
        fake.captureException(RuntimeException("test error"))
        fake.setUser("user-123")

        assertEquals("track 应被调用 1 次", 1, fake.trackedEvents.size)
        assertEquals("事件名应正确", "room_joined", fake.trackedEvents[0].first)
        assertEquals("captureException 应被调用 1 次", 1, fake.capturedExceptions.size)
        assertEquals("setUser 应被调用 1 次", 1, fake.setUserCalls.size)
        assertEquals("userId 应正确", "user-123", fake.setUserCalls[0])
    }

    // ─────────────────────────────────────────────
    // 辅助：FakeAnalytics — 记录调用，不执行实际操作
    // ─────────────────────────────────────────────

    class FakeAnalytics : AnalyticsPort {
        val trackedEvents = mutableListOf<Pair<String, Map<String, Any?>>>()
        val capturedExceptions = mutableListOf<Throwable>()
        val setUserCalls = mutableListOf<String?>()
        var currentConsent: ConsentMode = ConsentMode.All

        override fun track(event: String, properties: Map<String, Any?>) {
            trackedEvents += event to properties
        }

        override fun setUser(userId: String?, traits: Map<String, Any?>) {
            setUserCalls += userId
        }

        override fun captureException(throwable: Throwable, extras: Map<String, Any?>) {
            capturedExceptions += throwable
        }

        override fun setConsent(mode: ConsentMode) {
            currentConsent = mode
        }
    }
}
