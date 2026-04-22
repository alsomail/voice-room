package com.voice.room.android.core.analytics

import com.voice.room.android.core.analytics.impl.SentryAnalytics
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * TDD 单元测试 — SentryAnalytics 防腐层行为 (T-30034)
 *
 * SA-01: captureException() 调用 SentryHub.capture（A34-02）
 * SA-02: captureException() extras 中的手机号经脱敏后传递（A34-05）
 * SA-03: captureException() extras 中的 JWT 经脱敏后传递（A34-06）
 * SA-04: ConsentMode.None 下 captureException() 不调用 SentryHub（A34-04 扩展）
 * SA-05: ConsentMode.CrashOnly 下 captureException() 仍调用 SentryHub（合规豁免）
 * SA-06: ConsentMode.CrashOnly 下 track() 不调用底层存储
 * SA-07: ConsentMode.All 下 track() 被正常记录
 * SA-08: setUser() 调用 SentryHub.setUser
 * SA-09: setUser(null) 调用 SentryHub.clearUser（登出场景）
 * SA-10: Throwable message 中手机号在传递前被脱敏
 * SA-11: captureException 后 capturedCount 正确递增
 * SA-12: 切换 ConsentMode 后行为立即生效
 */
class SentryAnalyticsTest {

    private lateinit var fakeSentryHub: FakeSentryHub
    private lateinit var filter: SensitiveFilter
    private lateinit var analytics: SentryAnalytics

    @Before
    fun setUp() {
        fakeSentryHub = FakeSentryHub()
        filter = SensitiveFilter()
        analytics = SentryAnalytics(filter = filter, sentryHub = fakeSentryHub)
    }

    // ─────────────────────────────────────────────
    // SA-01: captureException() 调用 SentryHub.capture
    // ─────────────────────────────────────────────

    @Test
    fun SA01_captureException_callsSentryHub() {
        val exception = RuntimeException("test crash")
        analytics.captureException(exception)

        assertEquals("captureException 应被调用 1 次", 1, fakeSentryHub.captureCount)
        assertNotNull("捕获的 Throwable 不应为 null", fakeSentryHub.lastCapturedException)
    }

    // ─────────────────────────────────────────────
    // SA-02: extras 中的手机号经脱敏后传递
    // ─────────────────────────────────────────────

    @Test
    fun SA02_captureException_phoneInExtras_isRedacted() {
        analytics.captureException(
            throwable = RuntimeException("error"),
            extras = mapOf("contact" to "+966512345678", "action" to "login")
        )

        val capturedExtras = fakeSentryHub.lastCapturedExtras
        assertNotNull("extras 不应为 null", capturedExtras)
        assertFalse(
            "手机号应在 extras 中被脱敏",
            capturedExtras?.get("contact")?.contains("+966512345678") ?: false
        )
        assertEquals("非敏感 extras 不应变化", "login", capturedExtras?.get("action"))
    }

    // ─────────────────────────────────────────────
    // SA-03: extras 中的 JWT 经脱敏后传递
    // ─────────────────────────────────────────────

    @Test
    fun SA03_captureException_jwtInExtras_isRedacted() {
        val jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.abc123def456"
        analytics.captureException(
            throwable = RuntimeException("token error"),
            extras = mapOf("token" to jwt)
        )

        val capturedExtras = fakeSentryHub.lastCapturedExtras
        assertFalse(
            "JWT 应在 extras 中被脱敏",
            capturedExtras?.get("token")?.contains(jwt) ?: false
        )
    }

    // ─────────────────────────────────────────────
    // SA-04: ConsentMode.None 下 captureException() 不调用 SentryHub
    // ─────────────────────────────────────────────

    @Test
    fun SA04_consentNone_captureException_doesNotCallHub() {
        analytics.setConsent(ConsentMode.None)
        analytics.captureException(RuntimeException("silent crash"))

        assertEquals("ConsentMode.None 下不应调用 SentryHub", 0, fakeSentryHub.captureCount)
    }

    // ─────────────────────────────────────────────
    // SA-05: ConsentMode.CrashOnly 下 captureException() 仍调用 SentryHub（合规豁免）
    // ─────────────────────────────────────────────

    @Test
    fun SA05_consentCrashOnly_captureException_stillCallsHub() {
        analytics.setConsent(ConsentMode.CrashOnly)
        analytics.captureException(RuntimeException("crash only mode"))

        assertEquals("CrashOnly 模式下 captureException 应仍然工作", 1, fakeSentryHub.captureCount)
    }

    // ─────────────────────────────────────────────
    // SA-06: ConsentMode.CrashOnly 下 track() 不调用底层
    // ─────────────────────────────────────────────

    @Test
    fun SA06_consentCrashOnly_track_isSkipped() {
        analytics.setConsent(ConsentMode.CrashOnly)
        analytics.track("room_joined", mapOf("roomId" to "r-001"))

        assertEquals("CrashOnly 模式下 track 应被跳过", 0, fakeSentryHub.breadcrumbCount)
    }

    // ─────────────────────────────────────────────
    // SA-07: ConsentMode.All 下 track() 被正常记录
    // ─────────────────────────────────────────────

    @Test
    fun SA07_consentAll_track_isCaptured() {
        analytics.setConsent(ConsentMode.All)
        analytics.track("gift_sent", mapOf("giftId" to "g-001"))

        assertEquals("ConsentMode.All 下 track 应记录为 breadcrumb", 1, fakeSentryHub.breadcrumbCount)
    }

    // ─────────────────────────────────────────────
    // SA-08: setUser() 调用 SentryHub.setUser
    // ─────────────────────────────────────────────

    @Test
    fun SA08_setUser_callsHub() {
        analytics.setUser("user-123", mapOf("nickname" to "Alice"))

        assertEquals("setUser 应被调用 1 次", 1, fakeSentryHub.setUserCount)
        assertEquals("userId 应正确传递", "user-123", fakeSentryHub.lastUserId)
    }

    // ─────────────────────────────────────────────
    // SA-09: setUser(null) 调用 SentryHub.clearUser（登出场景）
    // ─────────────────────────────────────────────

    @Test
    fun SA09_setUser_null_callsClearUser() {
        analytics.setUser(null)

        assertEquals("clearUser 应被调用 1 次", 1, fakeSentryHub.clearUserCount)
    }

    // ─────────────────────────────────────────────
    // SA-10: Throwable message 中手机号在传递前被脱敏
    // ─────────────────────────────────────────────

    @Test
    fun SA10_captureException_throwableMessagePhone_isRedacted() {
        val exception = RuntimeException("用户 +966512345678 登录失败")
        analytics.captureException(exception)

        val capturedThrowable = fakeSentryHub.lastCapturedException
        assertNotNull("捕获的 Throwable 不应为 null", capturedThrowable)
        assertFalse(
            "Throwable message 中手机号应被脱敏",
            capturedThrowable?.message?.contains("+966512345678") ?: false
        )
    }

    // ─────────────────────────────────────────────
    // SA-11: captureException 后 capturedCount 正确递增
    // ─────────────────────────────────────────────

    @Test
    fun SA11_captureException_countIncrementsCorrectly() {
        repeat(3) { i ->
            analytics.captureException(RuntimeException("error $i"))
        }
        assertEquals("连续 3 次捕获，count 应为 3", 3, fakeSentryHub.captureCount)
    }

    // ─────────────────────────────────────────────
    // SA-12: 切换 ConsentMode 后行为立即生效
    // ─────────────────────────────────────────────

    @Test
    fun SA12_consentSwitch_behaviorChangesImmediately() {
        // 初始 All 模式：track 有效
        analytics.setConsent(ConsentMode.All)
        analytics.track("event_1")
        assertEquals("All 模式应记录 breadcrumb", 1, fakeSentryHub.breadcrumbCount)

        // 切换 CrashOnly：track 被跳过
        analytics.setConsent(ConsentMode.CrashOnly)
        analytics.track("event_2")
        assertEquals("CrashOnly 模式不应增加 breadcrumb", 1, fakeSentryHub.breadcrumbCount)

        // 切换回 All：track 恢复
        analytics.setConsent(ConsentMode.All)
        analytics.track("event_3")
        assertEquals("切换回 All 后 breadcrumb 应增加", 2, fakeSentryHub.breadcrumbCount)
    }

    // ─────────────────────────────────────────────
    // FakeSentryHub — 记录调用，不实际连接 Sentry 服务器
    // ─────────────────────────────────────────────

    class FakeSentryHub : SentryAnalytics.SentryHub {
        var captureCount = 0
        var lastCapturedException: Throwable? = null
        var lastCapturedExtras: Map<String, String?>? = null

        var breadcrumbCount = 0
        var lastBreadcrumbMessage: String? = null

        var setUserCount = 0
        var lastUserId: String? = null

        var clearUserCount = 0

        override fun captureException(throwable: Throwable, extras: Map<String, String?>) {
            captureCount++
            lastCapturedException = throwable
            lastCapturedExtras = extras
        }

        override fun addBreadcrumb(message: String, category: String) {
            breadcrumbCount++
            lastBreadcrumbMessage = message
        }

        override fun setUser(userId: String, traits: Map<String, Any?>) {
            setUserCount++
            lastUserId = userId
        }

        override fun clearUser() {
            clearUserCount++
        }
    }
}
