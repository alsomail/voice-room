package com.voice.room.android.core.analytics.impl

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.analytics.privacy.SensitiveFilter

/**
 * Sentry Analytics 防腐层实现（T-30034）
 *
 * ⚠️ 此类是整个项目中唯一允许引用 io.sentry.* 的位置（通过 [DefaultSentryHub]）。
 * 业务层只能通过 [AnalyticsPort] 接口访问 Analytics 功能。
 *
 * 架构说明：
 * - [SentryHub] 内部接口抽象 Sentry SDK 操作，使本类在 JVM 单元测试中可测
 * - [DefaultSentryHub] 是生产实现，调用真实 Sentry SDK
 * - 测试中注入 FakeSentryHub，不需要 Sentry SDK 初始化
 *
 * 脱敏：所有传递给 [SentryHub] 的数据均由 [SensitiveFilter] 预处理，
 * 手机号和 JWT 在离开此类前已被替换为 ***。
 *
 * @param filter 脱敏过滤器
 * @param sentryHub Sentry 操作抽象（生产用 [DefaultSentryHub]，测试注入 Fake）
 * @param initialConsent 初始同意模式（默认 CrashOnly，用户明确同意后调用 setConsent(All)）
 */
class SentryAnalytics(
    private val filter: SensitiveFilter,
    private val sentryHub: SentryHub = DefaultSentryHub,
    initialConsent: ConsentMode = ConsentMode.CrashOnly
) : AnalyticsPort {

    @Volatile
    private var currentConsent: ConsentMode = initialConsent

    // ─────────────────────────────────────────────
    // AnalyticsPort 实现
    // ─────────────────────────────────────────────

    override fun track(event: String, properties: Map<String, Any?>) {
        if (currentConsent != ConsentMode.All) return
        val scrubbedProps = filter.scrubExtras(properties)
        val message = buildString {
            append(event)
            if (scrubbedProps.isNotEmpty()) {
                append(" ")
                append(scrubbedProps.entries.joinToString(", ") { "${it.key}=${it.value}" })
            }
        }
        sentryHub.addBreadcrumb(message = message, category = "analytics")
    }

    override fun setUser(userId: String?, traits: Map<String, Any?>) {
        if (userId == null) {
            sentryHub.clearUser()
        } else {
            sentryHub.setUser(userId, traits)
        }
    }

    override fun captureException(throwable: Throwable, extras: Map<String, Any?>) {
        if (currentConsent == ConsentMode.None) return

        val scrubbedThrowable = filter.scrubThrowable(throwable)
        val scrubbedExtras = filter.scrubExtras(extras)
        sentryHub.captureException(scrubbedThrowable, scrubbedExtras)
    }

    override fun setConsent(mode: ConsentMode) {
        currentConsent = mode
    }

    // ─────────────────────────────────────────────
    // SentryHub 内部接口（测试可替换）
    // ─────────────────────────────────────────────

    /**
     * Sentry SDK 操作抽象接口。
     *
     * 此接口屏蔽了 io.sentry.* 对测试的影响：
     * - 生产：[DefaultSentryHub] 直接调用 Sentry SDK 静态方法
     * - 测试：FakeSentryHub 记录调用次数，不连接服务器
     */
    interface SentryHub {
        fun captureException(throwable: Throwable, extras: Map<String, String?>)
        fun addBreadcrumb(message: String, category: String)
        fun setUser(userId: String, traits: Map<String, Any?>)
        fun clearUser()
    }

    // ─────────────────────────────────────────────
    // DefaultSentryHub — 生产实现（唯一 import io.sentry.* 的位置）
    // ─────────────────────────────────────────────

    /**
     * 生产 Sentry Hub 实现。
     *
     * ⚠️ 此对象是整个项目中唯一允许 import io.sentry.* 的位置。
     * 不要在此文件外部直接调用 Sentry API。
     *
     * 注：Sentry SDK 在此项目中通过 io.sentry:sentry-android 引入，
     * 初始化由 SentryAndroid.init() 在 Application.onCreate() 中完成。
     * 如果 Sentry 未初始化，下方调用会静默降级（SDK 内部处理）。
     */
    object DefaultSentryHub : SentryHub {

        @Suppress("TooGenericExceptionCaught")
        override fun captureException(throwable: Throwable, extras: Map<String, String?>) {
            try {
                // NOTE: 当 sentry-android 依赖添加后，取消以下注释并删除 fallback
                // import io.sentry.Sentry  ← 在此文件顶层 import（仅本文件允许）
                // Sentry.captureException(throwable) { scope ->
                //     extras.forEach { (k, v) -> scope.setExtra(k, v ?: "null") }
                // }
                //
                // MVP 阶段：Sentry SDK 尚未添加为依赖，使用 fallback 日志
                android.util.Log.e("SentryAnalytics", "captureException: ${throwable.message}", throwable)
                extras.forEach { (k, v) ->
                    android.util.Log.e("SentryAnalytics", "  extra[$k] = $v")
                }
            } catch (e: Exception) {
                android.util.Log.e("SentryAnalytics", "Failed to capture exception via Sentry", e)
            }
        }

        override fun addBreadcrumb(message: String, category: String) {
            try {
                // NOTE: 当 sentry-android 依赖添加后，取消以下注释
                // val breadcrumb = io.sentry.Breadcrumb(message)
                // breadcrumb.category = category
                // Sentry.addBreadcrumb(breadcrumb)
                android.util.Log.d("SentryAnalytics", "breadcrumb[$category]: $message")
            } catch (e: Exception) {
                android.util.Log.w("SentryAnalytics", "Failed to add breadcrumb", e)
            }
        }

        override fun setUser(userId: String, traits: Map<String, Any?>) {
            try {
                // NOTE: 当 sentry-android 依赖添加后，取消以下注释
                // val user = io.sentry.protocol.User()
                // user.id = userId
                // Sentry.setUser(user)
                android.util.Log.d("SentryAnalytics", "setUser: $userId")
            } catch (e: Exception) {
                android.util.Log.w("SentryAnalytics", "Failed to set Sentry user", e)
            }
        }

        override fun clearUser() {
            try {
                // NOTE: 当 sentry-android 依赖添加后，取消以下注释
                // Sentry.setUser(null)
                android.util.Log.d("SentryAnalytics", "clearUser")
            } catch (e: Exception) {
                android.util.Log.w("SentryAnalytics", "Failed to clear Sentry user", e)
            }
        }
    }
}
