package com.voice.room.android.core.ws

import com.google.gson.GsonBuilder
import java.util.UUID

/**
 * C2S WebSocket 信令信封（与 server 端 `IncomingMessage` 结构对齐）。
 *
 * 服务端反序列化时严格读取：
 * - `type`（顶层字符串）
 * - `msg_id`（snake_case；可选幂等 ID）
 * - `payload`（嵌套对象，业务字段全部 snake_case）
 * - `timestamp`（毫秒）
 *
 * 通过此信封统一序列化所有上行信令，避免：
 * 1. 字段名错位（roomId → room_id）
 * 2. 字符串拼接 JSON 注入（reason 含 `"` 等特殊字符破坏报文）
 *
 * 内部使用 Gson `disableHtmlEscaping()` 但保留默认字符串转义（处理 `\n` `"` `\` Unicode 等）。
 *
 * ※ 不在 Json 中输出 `null` 字段（[GsonBuilder.serializeNulls] 关闭，默认即可）。
 */
object WsEnvelope {
    private val gson = GsonBuilder()
        .disableHtmlEscaping()
        .create()

    /**
     * 构建 C2S 信封 JSON 文本。
     *
     * @param type     信令类型（如 "JoinRoom" / "TakeMic" / "SendMessage"）
     * @param payload  业务字段映射（key 必须为 snake_case，value 任意 Gson 可序列化对象）
     * @param msgId    可选自定义 msg_id；不传则随机 UUID
     * @return 形如 `{"type":"JoinRoom","msg_id":"…","payload":{…},"timestamp":…}` 的 JSON 字符串
     */
    @JvmStatic
    fun build(
        type: String,
        payload: Map<String, Any?> = emptyMap(),
        msgId: String? = null,
    ): String {
        val envelope = linkedMapOf<String, Any?>(
            "type" to type,
            "msg_id" to (msgId ?: UUID.randomUUID().toString()),
            "payload" to payload,
            "timestamp" to System.currentTimeMillis(),
        )
        return gson.toJson(envelope)
    }
}

/**
 * 便捷扩展：发送一个 C2S 信封（type + payload）。
 *
 * 调用点示例：
 * ```
 * wsClient.sendEnvelope("TakeMic", mapOf("mic_index" to slotIndex))
 * ```
 *
 * @return [IWebSocketClient.send] 的返回值（true=已入队，false=连接未就绪）
 */
fun IWebSocketClient.sendEnvelope(
    type: String,
    payload: Map<String, Any?> = emptyMap(),
    msgId: String? = null,
): Boolean = send(WsEnvelope.build(type, payload, msgId))
