package com.voice.room.android.core.analytics.wire

import com.google.gson.Gson
import com.voice.room.android.core.analytics.queue.EventQueueEntity

/**
 * 事件上报 Wire Schema 单一事实源（T-30035 / 模块 7 R1 缺陷 1 修复）
 *
 * 严格对齐服务端 `app/server/src/core/analytics/writer.rs::EventInput`（T-00022/T-00023）：
 *
 * ```json
 * {
 *   "event_name": "...",          // 必填
 *   "device_id":  "...",          // 必填，不可为空字符串
 *   "user_id":    "uuid|null",
 *   "session_id": "...",
 *   "client_ts":  1720000000000,
 *   "properties": { ... },        // 必须是对象，不能是字符串
 *   "app_version":  "1.2.0",
 *   "os_version":   "Android 14",
 *   "locale":       "ar-SA",
 *   "network_type": "WIFI"
 * }
 * ```
 *
 * 本文件**不允许**业务层 import；仅供 [com.voice.room.android.core.analytics.transport]
 * 包内的 `WsTransport` / `HttpTransport` 调用。
 */
object EventWire {

    /**
     * 将单个 [EventQueueEntity] 序列化为符合服务端 `EventInput` schema 的 Map。
     *
     * 注意：`properties` 字段必须是 **对象**（Map），不能是字符串；
     * 因此此处会先将 `propertiesJson` 反序列化回 Map 再嵌入。
     */
    fun toEventMap(entity: EventQueueEntity, gson: Gson): Map<String, Any?> {
        val propsObject: Map<String, Any?> = parsePropertiesJson(entity.propertiesJson, gson)
        return linkedMapOf(
            "event_name" to entity.eventName,
            "device_id" to entity.deviceId,
            "user_id" to entity.userId,
            "session_id" to entity.sessionId,
            "client_ts" to entity.clientTs,
            "properties" to propsObject,
            "app_version" to entity.appVersion,
            "os_version" to entity.osVersion,
            "locale" to entity.locale,
            "network_type" to entity.networkType
        )
    }

    /**
     * 构造 HTTP `POST /api/v1/events/batch` 请求体：
     * `{ "events": [ {...}, ... ] }`
     */
    fun toHttpBody(batch: List<EventQueueEntity>, gson: Gson): Map<String, Any?> =
        mapOf("events" to batch.map { toEventMap(it, gson) })

    /**
     * 构造 WS `ReportEvent` 信令（protocol §6.3）：
     *
     * ```json
     * {
     *   "type":   "ReportEvent",
     *   "msg_id": "<uuid>",
     *   "payload": { "events": [ {...}, ... ] }
     * }
     * ```
     *
     * 严格遵守 `payload = { "events": [...] }` 结构（与服务端 `ws.rs:80` 对齐），
     * 且必须携带 `msg_id`。
     */
    fun toWsEnvelope(
        batch: List<EventQueueEntity>,
        msgId: String,
        gson: Gson
    ): Map<String, Any?> = mapOf(
        "type" to "ReportEvent",
        "msg_id" to msgId,
        "payload" to mapOf("events" to batch.map { toEventMap(it, gson) })
    )

    /**
     * 将 propertiesJson 反序列化为 Map<String, Any?>。
     * 失败时返回空 Map（绝不返回 String 标量，避免 JSONB 类型污染）。
     */
    private fun parsePropertiesJson(json: String, gson: Gson): Map<String, Any?> {
        if (json.isBlank()) return emptyMap()
        return try {
            @Suppress("UNCHECKED_CAST")
            (gson.fromJson(json, Map::class.java) as? Map<String, Any?>) ?: emptyMap()
        } catch (e: Exception) {
            emptyMap()
        }
    }
}
