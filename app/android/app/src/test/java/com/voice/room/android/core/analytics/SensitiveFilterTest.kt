package com.voice.room.android.core.analytics

import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * TDD 单元测试 — SensitiveFilter 脱敏逻辑 (T-30034)
 *
 * SF-01: 国际格式手机号 +966512345678 被替换为 ***
 * SF-02: 本地格式手机号 05012345678 被替换为 ***
 * SF-03: JWT eyJxxx.yyy.zzz 被替换为 ***
 * SF-04: 普通文本不受影响，原样返回
 * SF-05: 同一字符串中多个敏感模式均被替换
 * SF-06: 空字符串安全处理，不抛异常
 * SF-07: null 值安全处理，不抛异常
 * SF-08: Throwable message 中的手机号被脱敏
 * SF-09: Throwable message 中的 JWT 被脱敏
 * SF-10: extras Map 中的手机号值被脱敏
 * SF-11: extras Map 中的 JWT 值被脱敏
 * SF-12: extras Map 中的安全值不受影响
 * SF-13: 特殊字符 / Unicode 文本不受影响
 * SF-14: 7 位以下数字不被当作手机号替换
 * SF-15: scrubExtras 对空 Map 安全处理
 */
class SensitiveFilterTest {

    private lateinit var filter: SensitiveFilter

    @Before
    fun setUp() {
        filter = SensitiveFilter()
    }

    // ─────────────────────────────────────────────
    // SF-01: 国际格式手机号替换
    // ─────────────────────────────────────────────

    @Test
    fun SF01_international_phoneNumber_isRedacted() {
        val input = "用户手机 +966512345678 已注册"
        val result = filter.scrubString(input)!!
        assertFalse("国际手机号应被脱敏", result.contains("+966512345678"))
        assertTrue("脱敏后应包含 ***", result.contains("***"))
    }

    // ─────────────────────────────────────────────
    // SF-02: 本地格式手机号替换（7-15 位纯数字）
    // ─────────────────────────────────────────────

    @Test
    fun SF02_local_phoneNumber_isRedacted() {
        val input = "phone: 05012345678"
        val result = filter.scrubString(input)!!
        assertFalse("本地手机号应被脱敏", result.contains("05012345678"))
        assertTrue("脱敏后应包含 ***", result.contains("***"))
    }

    // ─────────────────────────────────────────────
    // SF-03: JWT 被替换
    // ─────────────────────────────────────────────

    @Test
    fun SF03_jwt_token_isRedacted() {
        val jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9" +
            ".eyJzdWIiOiIxMjM0NTY3ODkwIn0" +
            ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
        val input = "Authorization: Bearer $jwt"
        val result = filter.scrubString(input)!!
        assertFalse("JWT 应被脱敏", result.contains(jwt))
        assertTrue("脱敏后应包含 ***", result.contains("***"))
    }

    // ─────────────────────────────────────────────
    // SF-04: 普通文本不受影响
    // ─────────────────────────────────────────────

    @Test
    fun SF04_plainText_isUnchanged() {
        val input = "用户点击了礼物按钮，房间ID: room-123"
        val result = filter.scrubString(input)
        assertEquals("普通文本不应被修改", input, result)
    }

    // ─────────────────────────────────────────────
    // SF-05: 多个敏感模式均被替换
    // ─────────────────────────────────────────────

    @Test
    fun SF05_multiplePatterns_allRedacted() {
        val jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.abc123def456"
        val input = "phone=+966512345678 token=$jwt"
        val result = filter.scrubString(input)!!
        assertFalse("手机号应被脱敏", result.contains("+966512345678"))
        assertFalse("JWT 应被脱敏", result.contains(jwt))
    }

    // ─────────────────────────────────────────────
    // SF-06: 空字符串安全处理
    // ─────────────────────────────────────────────

    @Test
    fun SF06_emptyString_isSafelyHandled() {
        val result = filter.scrubString("")
        assertEquals("空字符串应返回空字符串", "", result)
    }

    // ─────────────────────────────────────────────
    // SF-07: null 值安全处理
    // ─────────────────────────────────────────────

    @Test
    fun SF07_nullValue_isSafelyHandled() {
        val result = filter.scrubString(null)
        assertEquals("null 应返回 null", null, result)
    }

    // ─────────────────────────────────────────────
    // SF-08: Throwable message 中的手机号被脱敏
    // ─────────────────────────────────────────────

    @Test
    fun SF08_throwableMessage_phoneRedacted() {
        val original = RuntimeException("验证码发送失败: +966512345678")
        val scrubbed = filter.scrubThrowable(original)
        assertFalse(
            "Throwable message 中手机号应被脱敏",
            scrubbed.message?.contains("+966512345678") ?: false
        )
        assertTrue("Throwable message 应包含 ***", scrubbed.message?.contains("***") ?: false)
    }

    // ─────────────────────────────────────────────
    // SF-09: Throwable message 中的 JWT 被脱敏
    // ─────────────────────────────────────────────

    @Test
    fun SF09_throwableMessage_jwtRedacted() {
        val jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.abc123def456"
        val original = RuntimeException("Token expired: $jwt")
        val scrubbed = filter.scrubThrowable(original)
        assertFalse(
            "Throwable message 中 JWT 应被脱敏",
            scrubbed.message?.contains(jwt) ?: false
        )
    }

    // ─────────────────────────────────────────────
    // SF-10: extras Map 中的手机号值被脱敏
    // ─────────────────────────────────────────────

    @Test
    fun SF10_extrasMap_phoneValueRedacted() {
        val extras = mapOf("phone" to "+966512345678", "roomId" to "room-123")
        val result = filter.scrubExtras(extras)
        assertFalse("extras 中手机号值应被脱敏", result["phone"]?.contains("+966512345678") ?: false)
        assertEquals("非敏感字段应不变", "room-123", result["roomId"])
    }

    // ─────────────────────────────────────────────
    // SF-11: extras Map 中的 JWT 值被脱敏
    // ─────────────────────────────────────────────

    @Test
    fun SF11_extrasMap_jwtValueRedacted() {
        val jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.abc123def456"
        val extras = mapOf("token" to jwt, "userId" to "user-456")
        val result = filter.scrubExtras(extras)
        assertFalse("extras 中 JWT 值应被脱敏", result["token"]?.contains(jwt) ?: false)
        assertEquals("非敏感字段应不变", "user-456", result["userId"])
    }

    // ─────────────────────────────────────────────
    // SF-12: extras Map 中的安全值不受影响
    // ─────────────────────────────────────────────

    @Test
    fun SF12_extrasMap_safeValues_unchanged() {
        val extras = mapOf(
            "event" to "gift_sent",
            "roomId" to "abc-123",
            "count" to "5"
        )
        val result = filter.scrubExtras(extras)
        assertEquals("event 不应变化", "gift_sent", result["event"])
        assertEquals("roomId 不应变化", "abc-123", result["roomId"])
        assertEquals("count 不应变化", "5", result["count"])
    }

    // ─────────────────────────────────────────────
    // SF-13: 特殊字符 / Unicode 文本不受影响
    // ─────────────────────────────────────────────

    @Test
    fun SF13_unicodeAndEmoji_isUnchanged() {
        val input = "مرحباً بكم 🎤 房间：متاح"
        val result = filter.scrubString(input)
        assertEquals("Unicode / emoji 文本不应被修改", input, result)
    }

    // ─────────────────────────────────────────────
    // SF-14: 7 位以下数字不被当作手机号
    // ─────────────────────────────────────────────

    @Test
    fun SF14_shortNumber_notRedacted() {
        val input = "版本号: 123456, 房间人数: 42"
        val result = filter.scrubString(input)!!
        assertTrue("短数字不应被脱敏", result.contains("42"))
        // 123456 是 6 位，也不应被脱敏
        assertTrue("6位数字不应被脱敏", result.contains("123456"))
    }

    // ─────────────────────────────────────────────
    // SF-15: scrubExtras 对空 Map 安全处理
    // ─────────────────────────────────────────────

    @Test
    fun SF15_emptyExtras_isSafelyHandled() {
        val result = filter.scrubExtras(emptyMap())
        assertTrue("空 extras 应返回空 Map", result.isEmpty())
    }
}
