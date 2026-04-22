package com.voice.room.android.core.analytics

/**
 * Analytics 防腐层接口（T-30034）
 *
 * 业务层通过此接口上报事件、设置用户、捕获崩溃。
 * 严禁业务层直接 import io.sentry.*，所有 Sentry 操作须经过此接口。
 *
 * 实现类：
 * - [impl.SentryAnalytics] — 生产实现，封装 Sentry SDK
 * - [impl.NoopAnalytics] — 空操作实现，用于测试或 CrashOnly 回退
 */
interface AnalyticsPort {

    /**
     * 上报业务事件
     * @param event 事件名，建议使用 [EventKey] 常量
     * @param properties 事件附加属性（可为空）
     */
    fun track(event: String, properties: Map<String, Any?> = emptyMap())

    /**
     * 设置当前用户信息（登录后调用）
     * @param userId 用户 ID，null 表示登出，清除用户身份
     * @param traits 用户属性（可为空）
     */
    fun setUser(userId: String?, traits: Map<String, Any?> = emptyMap())

    /**
     * 捕获异常并上报
     * 脱敏在实现层自动处理，业务层无需手动脱敏。
     * @param throwable 要捕获的异常
     * @param extras 附加上下文（手机号/JWT 会在实现层自动脱敏）
     */
    fun captureException(throwable: Throwable, extras: Map<String, Any?> = emptyMap())

    /**
     * 设置用户同意模式（合规管理）
     *
     * - [ConsentMode.All] 全量上报（事件 + Crash）
     * - [ConsentMode.CrashOnly] 仅 Crash 上报（合规豁免，Crash 不需用户同意）
     * - [ConsentMode.None] 完全禁用
     */
    fun setConsent(mode: ConsentMode)
}

/**
 * 用户数据同意模式（T-30034 合规豁免）
 *
 * 即使用户未同意全量 Analytics，CrashOnly 模式下 Crash 上报仍开启，
 * 符合 PDPL（沙特个人数据保护法）的合规豁免条款。
 */
enum class ConsentMode {
    /** 全量上报：事件追踪 + 崩溃上报均开启 */
    All,

    /**
     * 仅崩溃上报：track() 事件被丢弃，captureException() 正常工作。
     * 适用于用户未同意 Analytics 但崩溃上报具有合规豁免的场景。
     */
    CrashOnly,

    /** 完全禁用：track() 和 captureException() 均不执行 */
    None
}
