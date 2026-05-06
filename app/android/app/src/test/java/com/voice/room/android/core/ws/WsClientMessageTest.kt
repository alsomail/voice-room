package com.voice.room.android.core.ws

import com.google.gson.JsonParser
import com.voice.room.android.core.ws.WsEnvelope
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * WsClientMessageTest — C→S 消息序列化验证
 *
 * 验证客户端发出的消息格式符合协议规范。
 * 重点：Ping 消息 type 字段必须为 "Ping"（大写）而非 "ping"（小写）。
 *
 * PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json
 */
class WsClientMessageTest {

    // ─── PING-1: Ping 消息序列化 type 大写 ───────────────────────────────────

    @Test
    fun `PING-1 Ping message serialized via WsEnvelope has type Ping uppercase`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json
        // {"type":"Ping","msg_id":"...","payload":{},"timestamp":...}
        val json = WsEnvelope.build("Ping")

        val obj = JsonParser.parseString(json).asJsonObject
        assertEquals(
            "type must be Ping (uppercase P), not ping",
            "Ping",
            obj.get("type").asString
        )
    }

    @Test
    fun `PING-2 Ping envelope contains msg_id field`() {
        val json = WsEnvelope.build("Ping")
        val obj = JsonParser.parseString(json).asJsonObject

        assertNotNull("msg_id field must be present in Ping envelope", obj.get("msg_id"))
        assertTrue(
            "msg_id must be non-empty string",
            obj.get("msg_id").asString.isNotEmpty()
        )
    }

    @Test
    fun `PING-3 Ping envelope contains timestamp field`() {
        val before = System.currentTimeMillis()
        val json = WsEnvelope.build("Ping")
        val after = System.currentTimeMillis()

        val obj = JsonParser.parseString(json).asJsonObject
        assertNotNull("timestamp field must be present", obj.get("timestamp"))
        val ts = obj.get("timestamp").asLong
        assertTrue("timestamp should be within bounds", ts in before..after)
    }

    // ─── JOINING / OTHER C→S ─────────────────────────────────────────────────

    @Test
    fun `C2S-1 JoinRoom envelope contains type and room_id in payload`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/JoinRoom.schema.json
        val json = WsEnvelope.build("JoinRoom", mapOf("room_id" to "room-abc"))
        val obj = JsonParser.parseString(json).asJsonObject

        assertEquals("type should be JoinRoom", "JoinRoom", obj.get("type").asString)
        val payload = obj.getAsJsonObject("payload")
        assertNotNull("payload must be present", payload)
        assertEquals(
            "payload.room_id should match",
            "room-abc",
            payload.get("room_id").asString
        )
    }

    @Test
    fun `C2S-2 TakeMic envelope contains mic_index in payload`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/TakeMic.schema.json
        val json = WsEnvelope.build("TakeMic", mapOf("mic_index" to 3))
        val obj = JsonParser.parseString(json).asJsonObject

        assertEquals("type should be TakeMic", "TakeMic", obj.get("type").asString)
        val payload = obj.getAsJsonObject("payload")
        assertEquals("payload.mic_index should be 3", 3, payload.get("mic_index").asInt)
    }

    @Test
    fun `C2S-3 SendMessage envelope contains content in payload`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/SendMessage.schema.json
        val json = WsEnvelope.build("SendMessage", mapOf("content" to "Hello"))
        val obj = JsonParser.parseString(json).asJsonObject

        assertEquals("type should be SendMessage", "SendMessage", obj.get("type").asString)
        val payload = obj.getAsJsonObject("payload")
        assertEquals(
            "payload.content should match",
            "Hello",
            payload.get("content").asString
        )
    }

    @Test
    fun `C2S-SPECIAL SendMessage with special chars does not break JSON`() {
        // Ensure JSON injection is prevented via Gson serialization
        val json = WsEnvelope.build("SendMessage", mapOf("content" to """He said "hello" & <world>"""))
        val obj = JsonParser.parseString(json)  // throws if invalid
        assertNotNull("JSON should be valid even with special characters", obj)
    }
}
