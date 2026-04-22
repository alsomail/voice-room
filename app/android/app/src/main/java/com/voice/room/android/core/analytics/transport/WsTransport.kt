package com.voice.room.android.core.analytics.transport

import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.google.gson.Gson

/**
 * WebSocket 事件上报实现（T-30035）
 *
 * WS 在线时（[IWebSocketClient.state] 为 [WebSocketState.Connected]）优先使用此传输。
 * 将批量事件序列化为 JSON ReportEvent 信令发送。
 *
 * 信令格式（见 doc/protocol/index.md）：
 * ```json
 * { "type": "ReportEvent", "payload": [ {...}, ... ] }
 * ```
 */
class WsTransport(
    private val wsClient: IWebSocketClient,
    private val gson: Gson = Gson()
) : Transport {

    /** 当前 WS 是否在线 */
    val isOnline: Boolean
        get() = wsClient.state.value is WebSocketState.Connected

    override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
        return try {
            val payload = batch.map { entity ->
                mapOf(
                    "event_name" to entity.eventName,
                    "properties" to entity.propertiesJson,
                    "session_id" to entity.sessionId,
                    "client_ts" to entity.clientTs
                )
            }
            val message = gson.toJson(
                mapOf("type" to "ReportEvent", "payload" to payload)
            )
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
