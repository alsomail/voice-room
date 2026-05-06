package com.voice.room.android.core.ws.model

import com.google.gson.Gson
import com.google.gson.GsonBuilder
import com.google.gson.JsonDeserializationContext
import com.google.gson.JsonDeserializer
import com.google.gson.JsonElement
import com.google.gson.JsonParseException
import java.lang.reflect.Type

/**
 * WsGsonFactory — 创建支持 WsServerMessage 多态反序列化的 Gson 实例（T-00101）
 *
 * 使用 [WsServerMessageTypeAdapter] 通过 "type" 字段识别子类型，
 * 避免 Gson 默认无法处理 sealed class 多态的问题。
 *
 * 使用方式：
 * ```kotlin
 * private val wsGson = WsGsonFactory.create()
 * val msg = wsGson.fromJson(raw, WsServerMessage::class.java)
 * ```
 */
object WsGsonFactory {
    /**
     * 创建含 [WsServerMessage] 多态适配器的 Gson 实例。
     *
     * 特性：
     * - 禁用 HTML 转义（保留 `&`, `<`, `>` 等字符原样）
     * - null 字段不序列化（节省带宽，客户端以默认值代替）
     */
    fun create(): Gson = GsonBuilder()
        .disableHtmlEscaping()
        .registerTypeHierarchyAdapter(
            WsServerMessage::class.java,
            WsServerMessageTypeAdapter()
        )
        .create()
}

/**
 * WsServerMessage 的自定义 JsonDeserializer。
 *
 * 读取 "type" 字段并将整个 JSON element 委托给对应的子类反序列化。
 *
 * 注意：内部使用独立的 [innerGson]（不含此 adapter）避免无限递归。
 */
internal class WsServerMessageTypeAdapter : JsonDeserializer<WsServerMessage> {

    /** 内部 Gson，不含 WsServerMessage adapter，避免无限递归 */
    private val innerGson: Gson = GsonBuilder()
        .disableHtmlEscaping()
        .create()

    override fun deserialize(
        json: JsonElement,
        typeOfT: Type,
        context: JsonDeserializationContext,
    ): WsServerMessage {
        val obj = if (json.isJsonObject) json.asJsonObject
            else throw JsonParseException("WsServerMessage expected a JSON object but got: $json")

        val type = obj.get("type")
            ?.takeIf { !it.isJsonNull }
            ?.asString
            ?: return WsServerMessage.Unknown("__missing_type__")

        return when (type) {
            // ── payload-nested (schema-conformant) ──────────────────────────
            "UserJoined"         -> innerGson.fromJson(json, WsServerMessage.UserJoined::class.java)
            "UserLeft"           -> innerGson.fromJson(json, WsServerMessage.UserLeft::class.java)
            "MicTaken"           -> innerGson.fromJson(json, WsServerMessage.MicTaken::class.java)
            "MicLeft"            -> innerGson.fromJson(json, WsServerMessage.MicLeft::class.java)
            "RoomMessage"        -> innerGson.fromJson(json, WsServerMessage.RoomMessage::class.java)
            "Pong"               -> innerGson.fromJson(json, WsServerMessage.Pong::class.java)
            "JoinRoomResult"     -> innerGson.fromJson(json, WsServerMessage.JoinRoomResult::class.java)
            "LeaveRoomResult"    -> innerGson.fromJson(json, WsServerMessage.LeaveRoomResult::class.java)
            "TakeMicResult"      -> innerGson.fromJson(json, WsServerMessage.TakeMicResult::class.java)
            "LeaveMicResult"     -> innerGson.fromJson(json, WsServerMessage.LeaveMicResult::class.java)
            "SendMessageResult"  -> innerGson.fromJson(json, WsServerMessage.SendMessageResult::class.java)
            "SendGiftResult"     -> innerGson.fromJson(json, WsServerMessage.SendGiftResult::class.java)
            "EventReportAck"     -> innerGson.fromJson(json, WsServerMessage.EventReportAck::class.java)
            "KickUserResult"     -> innerGson.fromJson(json, WsServerMessage.KickUserResult::class.java)
            "MuteUserResult"     -> innerGson.fromJson(json, WsServerMessage.MuteUserResult::class.java)
            "UnmuteUserResult"   -> innerGson.fromJson(json, WsServerMessage.UnmuteUserResult::class.java)
            "TransferAdminResult"  -> innerGson.fromJson(json, WsServerMessage.TransferAdminResult::class.java)
            "ForceTakeMicResult"   -> innerGson.fromJson(json, WsServerMessage.ForceTakeMicResult::class.java)
            "ForceLeaveMicResult"  -> innerGson.fromJson(json, WsServerMessage.ForceLeaveMicResult::class.java)

            // ── flat / backward-compat ───────────────────────────────────────
            "UserMuted"          -> innerGson.fromJson(json, WsServerMessage.UserMuted::class.java)
            "AdminChanged"       -> innerGson.fromJson(json, WsServerMessage.AdminChanged::class.java)
            "RoomInfoUpdated"    -> innerGson.fromJson(json, WsServerMessage.RoomInfoUpdated::class.java)
            "GiftReceived"       -> innerGson.fromJson(json, WsServerMessage.GiftReceived::class.java)
            "UserKicked"         -> innerGson.fromJson(json, WsServerMessage.UserKicked::class.java)
            "MessageReceived"    -> innerGson.fromJson(json, WsServerMessage.MessageReceived::class.java)
            "RoomClosed"         -> WsServerMessage.RoomClosed
            "Error"              -> innerGson.fromJson(json, WsServerMessage.ServerError::class.java)

            // ── catchall ────────────────────────────────────────────────────
            else                 -> WsServerMessage.Unknown(type)
        }
    }
}
