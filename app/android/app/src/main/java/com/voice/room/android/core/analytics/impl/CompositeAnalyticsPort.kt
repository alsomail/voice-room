package com.voice.room.android.core.analytics.impl

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.analytics.EventReportClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * 复合 [AnalyticsPort] 实现（T-30035 / R1 批 2 缺陷 2）
 *
 * 将业务层的 [track] 调用同时分发给：
 * 1. **下游 Analytics**（典型为 [SentryAnalytics]）— 写 breadcrumb / setUser
 * 2. **[EventReportClient]** — 入队 + 节流 flush 到 App Server `/api/v1/events/batch`
 *
 * 此适配器位于 `core.analytics.impl` 包，**禁止** 被业务层（presentation/feature/data 等）
 * 直接 import；业务层只持有 [AnalyticsPort] 抽象。
 *
 * **同步语义**：[track] 非 suspend，内部通过 [scope] 异步入队，避免阻塞 UI 线程；
 * 队列写入失败会被静默吞掉，由 [EventReportClient.flush] 的失败路径补偿（Sentry breadcrumb）。
 *
 * **隐私合规**：[ConsentMode] 由 [EventReportClient] 本身门控（CrashOnly / None 时直接丢弃事件）；
 * 此处无需重复检查。
 *
 * @param downstream     下游 Sentry 实现（断路时可注 [NoopAnalytics]）
 * @param eventReporter  事件上报客户端（HTTP + WS 双通道）
 * @param scope          入队协程作用域（由 AppContainer 持有 SupervisorJob + Dispatchers.IO）
 */
class CompositeAnalyticsPort(
    private val downstream: AnalyticsPort,
    private val eventReporter: EventReportClient,
    private val scope: CoroutineScope
) : AnalyticsPort {

    override fun track(event: String, properties: Map<String, Any?>) {
        // 1) 同步走 Sentry breadcrumb（CrashOnly 时 SentryAnalytics 内部会丢弃 track）
        downstream.track(event, properties)
        // 2) 异步入队到 EventReportClient（CrashOnly/None 时其内部直接丢弃）
        scope.launch {
            eventReporter.track(event, properties)
        }
    }

    override fun setUser(userId: String?, traits: Map<String, Any?>) {
        downstream.setUser(userId, traits)
    }

    override fun captureException(throwable: Throwable, extras: Map<String, Any?>) {
        downstream.captureException(throwable, extras)
    }

    override fun setConsent(mode: ConsentMode) {
        downstream.setConsent(mode)
    }
}
