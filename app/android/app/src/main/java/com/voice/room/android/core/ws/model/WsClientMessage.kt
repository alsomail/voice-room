package com.voice.room.android.core.ws.model

import com.google.gson.annotations.SerializedName
import java.util.UUID

/**
 * C→S WebSocket 信令类型安全模型（T-00101 P1-5）
 *
 * 所有 C→S 信令均通过 [WsEnvelope.build] 序列化后发出；
 * 本 sealed class 提供类型安全的信令描述符，避免散落的魔法字符串。
 *
 * 使用方式：
 * ```kotlin
 * // 直接通过 WsEnvelope（已有实现，向后兼容）
 * wsClient.sendEnvelope("TakeMic", mapOf("mic_index" to 3))
 *
 * // 或使用类型安全包装（推荐新代码）
 * wsClient.send(WsClientMessage.TakeMic(micIndex = 3).toEnvelopeJson())
 * ```
 *
 * ## C→S 信令文档
 * 所有字段对齐服务端 `IncomingMessage` 反序列化格式（snake_case payload）。
 *
 * PROTO-BINDING: doc/protocol/schemas/ws/ (C→S signals)
 */
sealed class WsClientMessage {

    /** 序列化为 WsEnvelope JSON，可直接传给 [IWebSocketClient.send] */
    abstract fun toEnvelopeJson(): String

    // ─── 心跳 ────────────────────────────────────────────────────────────────

    /**
     * 心跳 Ping（大写 P 对齐服务端协议）。
     * PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json
     */
    data class Ping(
        @SerializedName("msg_id") val msgId: String = UUID.randomUUID().toString(),
    ) : WsClientMessage() {
        override fun toEnvelopeJson(): String =
            com.voice.room.android.core.ws.WsEnvelope.build("Ping", msgId = msgId)
    }

    // ─── 房间生命周期 ─────────────────────────────────────────────────────────

    /**
     * 加入房间。
     * PROTO-BINDING: doc/protocol/schemas/ws/JoinRoom.schema.json
     */
    data class JoinRoom(
        @SerializedName("room_id") val roomId: String,
        val token: String? = null,
        @SerializedName("last_msg_id") val lastMsgId: String? = null,
    ) : WsClientMessage() {
        override fun toEnvelopeJson(): String = com.voice.room.android.core.ws.WsEnvelope.build(
            "JoinRoom",
            buildMap {
                put("room_id", roomId)
                if (token != null) put("token", token)
                if (lastMsgId != null) put("last_msg_id", lastMsgId)
            },
        )
    }

    /**
     * 离开房间。
     * PROTO-BINDING: doc/protocol/schemas/ws/LeaveRoom.schema.json
     */
    data class LeaveRoom(
        @SerializedName("room_id") val roomId: String,
    ) : WsClientMessage() {
        override fun toEnvelopeJson(): String = com.voice.room.android.core.ws.WsEnvelope.build(
            "LeaveRoom",
            mapOf("room_id" to roomId),
        )
    }

    // ─── 麦位 ────────────────────────────────────────────────────────────────

    /**
     * 上麦请求。
     * PROTO-BINDING: doc/protocol/schemas/ws/TakeMic.schema.json
     */
    data class TakeMic(
        @SerializedName("mic_index") val micIndex: Int,
    ) : WsClientMessage() {
        override fun toEnvelopeJson(): String = com.voice.room.android.core.ws.WsEnvelope.build(
            "TakeMic",
            mapOf("mic_index" to micIndex),
        )
    }

    /**
     * 下麦请求。
     * PROTO-BINDING: doc/protocol/schemas/ws/LeaveMic.schema.json
     */
    data object LeaveMic : WsClientMessage() {
        override fun toEnvelopeJson(): String =
            com.voice.room.android.core.ws.WsEnvelope.build("LeaveMic")
    }

    // ─── 聊天消息 ─────────────────────────────────────────────────────────────

    /**
     * 发送房间文本消息。
     * PROTO-BINDING: doc/protocol/schemas/ws/SendMessage.schema.json
     */
    data class SendMessage(
        val content: String,
    ) : WsClientMessage() {
        override fun toEnvelopeJson(): String = com.voice.room.android.core.ws.WsEnvelope.build(
            "SendMessage",
            mapOf("content" to content),
        )
    }
}
