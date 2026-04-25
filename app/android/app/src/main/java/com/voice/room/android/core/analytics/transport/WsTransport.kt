package com.voice.room.android.core.analytics.transport

import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.wire.EventWire
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.google.gson.Gson
import java.util.UUID

/**
 * WebSocket 事件上报实现（T-30035）
 *
 * WS 在线时（[IWebSocketClient.state] 为 [WebSocketState.Connected]）优先使用此传输。
 * 将批量事件序列化为 JSON ReportEvent 信令发送。
 *
 * R1 修复（缺陷 1）：信令格式严格遵守 protocol §6.3，使用 [EventWire] 单一事实源构造
 * `payload = { "events": [...] }` + `msg_id`；事件对象的 properties 字段为 Map 不为标量字符串。
 *
 * ```json
 * { "type": "ReportEvent", "msg_id": "<uuid>", "payload": { "events": [ {...} ] } }
 * ```
 *
 * @param msgIdGen msg_id 生成器（默认 UUIDv4，测试可注入固定值）
 */
class WsTransport(
    private val wsClient: IWebSocketClient,
    private val gson: Gson = Gson(),
    private val msgIdGen: () -> String = { UUID.randomUUID().toString() }
) : Transport {

    /** 当前 WS 是否在线 */
    val isOnline: Boolean
        get() = wsClient.state.value is WebSocketState.Connected

    override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
        return try {
            val envelope = EventWire.toWsEnvelope(batch, msgIdGen(), gson)
            val message = gson.toJson(envelope)
            val sent = wsClient.send(message)
            if (sent) {
                Result.success(SendOutcome(batch.map { it.id }))
            } else {
                Result.failure(IllegalStateException("WS send returned false"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
