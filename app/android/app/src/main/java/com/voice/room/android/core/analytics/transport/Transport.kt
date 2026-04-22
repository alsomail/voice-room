package com.voice.room.android.core.analytics.transport

import com.voice.room.android.core.analytics.queue.EventQueueEntity

/**
 * 事件上报结果（T-30035）
 *
 * @param acceptedIds 服务端已确认接收的事件 id 列表（可从队列中删除）
 */
data class SendOutcome(val acceptedIds: List<Long>)

/**
 * 事件批量上报传输层接口（T-30035）
 *
 * 两种实现：
 * - [WsTransport]：WebSocket 在线优先
 * - [HttpTransport]：HTTP fallback（WS 离线时使用）
 */
interface Transport {
    /**
     * 批量发送事件
     * @return [Result.success] 含 [SendOutcome]；网络错误时 [Result.failure]
     */
    suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome>
}
