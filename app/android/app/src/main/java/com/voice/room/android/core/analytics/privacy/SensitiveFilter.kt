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

    /** 手机号正则：可选 + 前缀，7–15 位数字（避免匹配短数字如版本号） */
    private val phoneRegex = Regex("""(\+\d{7,15}|\b\d{10,15}\b)""")

    /** JWT 正则：eyJ 开头的三段 Base64URL */
    private val jwtRegex = Regex("""eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+""")

    private val redacted = "***"

    /**
     * 对字符串进行脱敏处理
     * @return 脱敏后的字符串；输入为 null 时返回 null
     */
    fun scrubString(input: String?): String? {
        if (input == null) return null
        return input
            .let { jwtRegex.replace(it, redacted) }
            .let { phoneRegex.replace(it, redacted) }
    }

    /**
     * 对 extras Map 中的所有字符串值进行脱敏
     * @return 脱敏后的 Map（key 不变，value 中的敏感信息被替换）
     */
    fun scrubExtras(extras: Map<String, Any?>): Map<String, String?> {
        return extras.mapValues { (_, value) ->
            scrubString(value?.toString())
        }
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
