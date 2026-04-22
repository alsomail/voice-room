package com.voice.room.android.core.analytics.impl

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode

/**
 * 空操作 Analytics 实现（T-30034）
 *
 * 用途：
 * - 单元测试注入：避免真实 Sentry/Analytics SDK 依赖
 * - 非生产环境兜底：当 Analytics 未初始化或被禁用时使用
 * - ConsentMode.None 模式下的回退实现
 *
 * 所有方法均为空操作，不执行任何实际记录。
 */
class NoopAnalytics : AnalyticsPort {

    override fun track(event: String, properties: Map<String, Any?>) {
        // 空操作，不上报任何事件
    }

    override fun setUser(userId: String?, traits: Map<String, Any?>) {
        // 空操作，不设置用户身份
    }

    override fun captureException(throwable: Throwable, extras: Map<String, Any?>) {
        // 空操作，不捕获任何异常
    }

    override fun setConsent(mode: ConsentMode) {
        // 空操作，忽略同意模式变更
    }
}
