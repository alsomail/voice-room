package com.voice.room.android.feature.auth

/**
 * 登录页 UI 状态 – 纯 Kotlin 数据类，无 Android 框架依赖，易于单元测试。
 *
 * @param phoneNumber       用户输入的手机号（不含 +966 前缀）
 * @param verificationCode  用户输入的 6 位验证码
 * @param countdownSeconds  发送验证码后的剩余倒计时秒数（0 表示未倒计时）
 * @param defaultCountryCode 默认国家码（沙特 +966）
 * @param isRtlLayout       是否强制 RTL 布局（阿拉伯语 / 沙特市场默认开启）
 *
 * T-30002 新增：
 * @param isLoading         登录接口请求进行中
 * @param isSendingCode     发送验证码接口请求进行中
 * @param error             接口返回的错误信息（null = 无错误）
 * @param isLoginSuccess    登录成功标志（token 已写入 DataStore）
 * @param isNewUser         true = 首次注册，可展示新手引导
 */
data class LoginUiState(
    val phoneNumber: String = "",
    val verificationCode: String = "",
    val countdownSeconds: Int = 0,
    val defaultCountryCode: String = "+966",
    val isRtlLayout: Boolean = true,
    // ── T-30002 新增字段 ──────────────────────────
    val isLoading: Boolean = false,
    val isSendingCode: Boolean = false,
    val error: String? = null,
    val isLoginSuccess: Boolean = false,
    val isNewUser: Boolean = false
) {
    /**
     * 发送验证码按钮是否可用：
     * - 手机号必须有效（9 位数字）
     * - 当前没有正在进行的倒计时
     * - 当前没有正在发送的请求
     */
    val isSendButtonEnabled: Boolean
        get() = isPhoneNumberValid(phoneNumber) && countdownSeconds == 0 && !isSendingCode

    /**
     * 是否正在倒计时（倒计时 > 0 即为倒计时中）
     */
    val isCountingDown: Boolean
        get() = countdownSeconds > 0

    /**
     * 倒计时按钮文案：倒计时中显示剩余秒数，否则返回空字符串
     */
    val countdownLabel: String
        get() = if (isCountingDown) "${countdownSeconds}s" else ""

    /**
     * 登录按钮是否可用：
     * - 手机号必须有效
     * - 验证码必须恰好 6 位
     * - 当前不在 Loading 状态
     */
    val isLoginButtonEnabled: Boolean
        get() = isPhoneNumberValid(phoneNumber)
                && verificationCode.length == VERIFICATION_CODE_LENGTH
                && !isLoading

    companion object {
        /** 发送验证码后的倒计时总秒数 */
        const val COUNTDOWN_SECONDS = 60

        /** 验证码位数 */
        const val VERIFICATION_CODE_LENGTH = 6

        /**
         * 沙特手机号验证：
         * 去除所有非数字字符后，须恰好为 9 位且以 '5' 开头
         * （+966 后的本机号格式，如 5XXXXXXXX）。
         */
        fun isPhoneNumberValid(phone: String): Boolean {
            val digits = phone.filter { it.isDigit() }
            return digits.length == 9 && digits.startsWith("5")
        }
    }
}
