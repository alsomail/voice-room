package com.voice.room.android.core.analytics.privacy

/**
 * 敏感数据脱敏过滤器（T-30034）
 *
 * 在 Sentry 上报前自动替换以下敏感模式：
 * - 手机号（国际格式 +XXXX... 或 7-15 位纯数字）
 * - JWT（eyJ... 三段式 Base64URL）
 *
 * 本类纯 Kotlin，无 Android 依赖，可在 JVM 单元测试中直接使用。
 */
class SensitiveFilter {

    /**
     * 国际格式手机号正则：必须以 `+` 开头且 7-15 位数字（如 +966512345678）。
     *
     * R1 修复（缺陷 9）：移除不带 `+` 的 `\b\d{10,15}\b` 整体扫描，
     * 因其会误判 13 位毫秒级时间戳（client_ts）、长 gift_id、订单号等数值字段。
     * 本地无 `+` 格式手机号改为 key-aware 扫描（见 [scrubExtras]）。
     */
    private val intlPhoneRegex = Regex("""\+\d{7,15}""")

    /**
     * 本地无 `+` 前缀手机号正则：仅在 key 命中 phone/mobile/tel 时启用。
     * 7-15 位数字（避免匹配短验证码、长订单号）。
     */
    private val localPhoneRegex = Regex("""\b\d{7,15}\b""")

    /** JWT 正则：eyJ 开头的三段 Base64URL */
    private val jwtRegex = Regex("""eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+""")

    /** key-aware 手机号字段名片段（不区分大小写） */
    private val phoneKeyHints = listOf("phone", "mobile", "tel")

    private val redacted = "***"

    /**
     * 对字符串进行脱敏处理（通用扫描：仅 JWT + 国际格式手机号）。
     * 用于自由文本场景（如 Throwable.message）。
     *
     * @return 脱敏后的字符串；输入为 null 时返回 null
     */
    fun scrubString(input: String?): String? {
        if (input == null) return null
        return input
            .let { jwtRegex.replace(it, redacted) }
            .let { intlPhoneRegex.replace(it, redacted) }
            .let { localPhoneRegex.replace(it, redacted) }
    }

    /**
     * 对 extras Map 中的 value 进行脱敏。
     *
     * R1 修复（缺陷 6）：保留原始类型 — 仅对 `String` 类型 value 执行字符串脱敏；
     * 数值/布尔/嵌套对象原样保留，避免 JSONB 数值字段语义丢失。
     *
     * R1 修复（缺陷 9）：key-aware 脱敏 — 普通字段仅扫描 JWT + 国际格式手机号；
     * 当 key 命中 `phone` / `mobile` / `tel` 时额外扫描本地无 `+` 手机号。
     *
     * @return 脱敏后的 Map（key 不变，value 类型保留）
     */
    fun scrubExtras(extras: Map<String, Any?>): Map<String, Any?> {
        return extras.mapValues { (key, value) ->
            if (value is String) {
                scrubValueByKey(key, value)
            } else {
                value
            }
        }
    }

    /**
     * 按 key 名称决定脱敏强度：
     * - 命中 phone/mobile/tel：通用 + 本地手机号
     * - 其它：仅 JWT + 国际手机号
     */
    private fun scrubValueByKey(key: String, value: String): String {
        val keyLower = key.lowercase()
        val isPhoneField = phoneKeyHints.any { keyLower.contains(it) }
        var out = jwtRegex.replace(value, redacted)
        out = intlPhoneRegex.replace(out, redacted)
        if (isPhoneField) {
            out = localPhoneRegex.replace(out, redacted)
        }
        return out
    }

    /**
     * 对 Throwable 的 message 进行脱敏，返回新的 RuntimeException
     * 保留原始 stacktrace 的 cause 链，但 message 被替换为脱敏版本。
     */
    fun scrubThrowable(throwable: Throwable): Throwable {
        val scrubbedMessage = scrubString(throwable.message) ?: throwable.message
        return RuntimeException(scrubbedMessage, throwable.cause).also { scrubbed ->
            scrubbed.stackTrace = throwable.stackTrace
        }
    }
}
