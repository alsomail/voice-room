package com.voice.room.android.core.analytics

import com.voice.room.android.core.analytics.context.CommonPropsProvider
import com.voice.room.android.core.analytics.queue.EventQueueDao
import com.voice.room.android.core.analytics.session.SessionManager
import com.voice.room.android.core.analytics.throttle.Throttler
import com.voice.room.android.core.analytics.transport.Transport
import com.voice.room.android.core.consent.ConsentRepository

/**
 * 事件上报客户端主入口（T-30035）
 *
 * 完整生命周期：
 * 1. `track()` → 检查 ConsentMode → 脱敏+注入公共属性 → 入队 → 通知 Throttler
 * 2. Throttler 条件满足 → `flush()` → WsTransport / HttpTransport → 成功删/失败保留
 *
 * 所有依赖通过构造注入，支持测试替换。
 *
 * @param queueDao        事件队列存储
 * @param throttler       flush 触发器（≥8 或 ≥2min）
 * @param wsTransport     WebSocket 传输（在线优先）
 * @param httpTransport   HTTP fallback 传输
 * @param consentRepo     同意模式管理
 * @param commonProps     公共属性注入器
 * @param sessionManager  session UUID 管理
 * @param analyticsPort   异常捕获上报（flush 失败时用）
 */
class EventReportClient(
    private val queueDao: EventQueueDao,
    private val throttler: Throttler,
    private val wsTransport: Transport,           // WsTransport
    private val httpTransport: Transport,         // HttpTransport fallback
    private val consentRepo: ConsentRepository,
    private val commonProps: CommonPropsProvider,
    private val sessionManager: SessionManager,
    private val analyticsPort: AnalyticsPort? = null,
    // 判断 WS 是否在线（抽象为 lambda 以避免直接依赖 WsTransport）
    private val isWsOnline: () -> Boolean = { false }
) {

    /**
     * 上报一个业务事件。
     *
     * - [ConsentMode.CrashOnly] / [ConsentMode.None]：立即返回，不入队
     * - [ConsentMode.All]：脱敏 → 注入公共属性 → 入队 → 触发 Throttler
     *
     * 此函数设计为非 suspend，可从任意线程安全调用；
     * 内部入队由协程完成（suspend DAO）。
     *
     * @param event      事件名（建议使用 [EventKey] 常量）
     * @param properties 事件附加属性
     */
    suspend fun track(event: String, properties: Map<String, Any?> = emptyMap()) {
        // E35-08: CrashOnly/None 时直接丢弃
        if (consentRepo.mode != ConsentMode.All) return

        val entity = commonProps.enrich(event, properties, sessionManager.currentId)
        queueDao.insert(entity)
        throttler.notify(queueDao.size())
    }

    /**
     * 将队列中的事件批量上报。
     *
     * 优先 WS；WS 离线时用 HTTP fallback。
     * 成功：从队列删除；失败：保留（由 Throttler 下次重试）。
     */
    suspend fun flush() {
        val batch = queueDao.getOldest(EventQueueDao.FLUSH_BATCH_SIZE)
        if (batch.isEmpty()) return

        val transport = if (isWsOnline()) wsTransport else httpTransport
        val result = transport.send(batch)

        result
            .onSuccess { outcome ->
                queueDao.deleteByIds(outcome.acceptedIds)
            }
            .onFailure { e ->
                analyticsPort?.captureException(
                    e, mapOf("flush_batch_size" to batch.size.toString())
                )
            }
    }

    companion object {
        /** Room 数据库表名（备用，给 Room 实现引用） */
        const val TABLE_NAME = "event_queue"
    }
}
