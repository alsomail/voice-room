package com.voice.room.android.core.ws

import com.voice.room.android.core.ws.model.WsGsonFactory
import com.voice.room.android.core.ws.model.WsServerMessage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.reflect.full.memberProperties

/**
 * PROTO-1~6: WsServerMessage sealed class 反序列化验证
 *
 * 验证新 sealed class 体系从真实协议 JSON envelope 正确解析，
 * 并断言旧版顶层 camelCase 字段（slotIndex / userId）不存在于 sealed class 字段中。
 *
 * PROTO-BINDING: doc/protocol/schemas/ws/
 */
class WsServerMessageTest {

    private val gson = WsGsonFactory.create()

    // ─── PROTO-1: MicTaken ────────────────────────────────────────────────────

    @Test
    fun `PROTO-1 MicTaken full envelope parses payload fields correctly`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
        val json = """
            {
              "type": "MicTaken",
              "payload": {
                "mic_index": 3,
                "user_id": "550e8400-e29b-41d4-a716-446655440000",
                "nickname": "Alice",
                "avatar": null
              },
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
              "timestamp": 1234567890
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to MicTaken", msg is WsServerMessage.MicTaken)
        val taken = msg as WsServerMessage.MicTaken
        assertEquals("payload.micIndex should be 3", 3, taken.payload.micIndex)
        assertEquals(
            "payload.userId should match uuid",
            "550e8400-e29b-41d4-a716-446655440000",
            taken.payload.userId
        )
        assertEquals("payload.nickname should be Alice", "Alice", taken.payload.nickname)
        assertNull("payload.avatar should be null", taken.payload.avatar)
        assertEquals(
            "msg_id should match",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            taken.msgId
        )
    }

    /**
     * PROTO-1-GUARD: WsServerMessage.MicTaken 不应有旧版顶层 slotIndex 字段。
     * 旧代码使用 json.get("slotIndex") 读取顶层字段，新代码应从 payload.mic_index 读取。
     */
    @Test
    fun `PROTO-1-GUARD MicTaken sealed class has no top-level slotIndex field`() {
        // Verify via reflection that WsServerMessage.MicTaken has no property named 'slotIndex'
        val propNames = WsServerMessage.MicTaken::class.memberProperties.map { it.name }
        assertFalse(
            "MicTaken must NOT have slotIndex property (should use payload.micIndex)",
            "slotIndex" in propNames
        )
        // And no top-level userId either (should be payload.userId)
        assertFalse(
            "MicTaken must NOT have top-level userId property (should use payload.userId)",
            "userId" in propNames
        )
        // But must have payload
        assertTrue(
            "MicTaken must have payload property",
            "payload" in propNames
        )
    }

    // ─── PROTO-2: MicLeft ─────────────────────────────────────────────────────

    @Test
    fun `PROTO-2 MicLeft full envelope parses payload fields correctly`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/MicLeft.schema.json
        val json = """
            {
              "type": "MicLeft",
              "payload": {
                "mic_index": 2,
                "user_id": "550e8400-e29b-41d4-a716-446655440001",
                "forced": true
              },
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9",
              "timestamp": 1234567891
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to MicLeft", msg is WsServerMessage.MicLeft)
        val left = msg as WsServerMessage.MicLeft
        assertEquals("payload.micIndex should be 2", 2, left.payload.micIndex)
        assertEquals(
            "payload.userId should match",
            "550e8400-e29b-41d4-a716-446655440001",
            left.payload.userId
        )
        assertEquals("payload.forced should be true", true, left.payload.forced)
    }

    @Test
    fun `PROTO-2-GUARD MicLeft sealed class has no top-level slotIndex field`() {
        val propNames = WsServerMessage.MicLeft::class.memberProperties.map { it.name }
        assertFalse(
            "MicLeft must NOT have slotIndex property (should use payload.micIndex)",
            "slotIndex" in propNames
        )
    }

    // ─── PROTO-3: UserJoined ─────────────────────────────────────────────────

    @Test
    fun `PROTO-3 UserJoined payload userId nickname avatar from nested payload`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json
        val json = """
            {
              "type": "UserJoined",
              "payload": {
                "user_id": "550e8400-e29b-41d4-a716-446655440002",
                "nickname": "Bob",
                "avatar": "https://example.com/avatar.png",
                "member_count": 5
              },
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430ca",
              "timestamp": 1234567892
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to UserJoined", msg is WsServerMessage.UserJoined)
        val joined = msg as WsServerMessage.UserJoined
        assertEquals(
            "payload.userId should match uuid",
            "550e8400-e29b-41d4-a716-446655440002",
            joined.payload.userId
        )
        assertEquals("payload.nickname should be Bob", "Bob", joined.payload.nickname)
        assertEquals(
            "payload.avatar should match url",
            "https://example.com/avatar.png",
            joined.payload.avatar
        )
        assertEquals("payload.memberCount should be 5", 5, joined.payload.memberCount)
    }

    /**
     * PROTO-3-GUARD: WsServerMessage.UserJoined 不应有旧版顶层 userId 字段。
     */
    @Test
    fun `PROTO-3-GUARD UserJoined has no top-level userId field`() {
        val propNames = WsServerMessage.UserJoined::class.memberProperties.map { it.name }
        assertFalse(
            "UserJoined must NOT have top-level userId property (should use payload.userId)",
            "userId" in propNames
        )
        assertTrue(
            "UserJoined must have payload property",
            "payload" in propNames
        )
    }

    @Test
    fun `PROTO-3b UserLeft parses payload userId`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json
        val json = """
            {
              "type": "UserLeft",
              "payload": {
                "user_id": "550e8400-e29b-41d4-a716-446655440003",
                "member_count": 4
              },
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cb",
              "timestamp": 1234567893
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to UserLeft", msg is WsServerMessage.UserLeft)
        val left = msg as WsServerMessage.UserLeft
        assertEquals(
            "payload.userId should match",
            "550e8400-e29b-41d4-a716-446655440003",
            left.payload.userId
        )
    }

    // ─── PROTO-4: UserMuted / AdminChanged / RoomInfoUpdated ─────────────────

    @Test
    fun `PROTO-4-a UserMuted flat format parses muteType and durationSec`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/UserMuted.schema.json (kept as flat for backward-compat)
        val expiresAt = 9999999999L
        val json = """
            {
              "type": "UserMuted",
              "muteType": "mic",
              "duration_sec": 600,
              "expires_at": $expiresAt,
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cc"
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to UserMuted", msg is WsServerMessage.UserMuted)
        val muted = msg as WsServerMessage.UserMuted
        assertEquals("muteType should be mic", "mic", muted.muteType)
        assertEquals("durationSec should be 600", 600, muted.durationSec)
        assertEquals("expiresAt should match", expiresAt, muted.expiresAt)
    }

    @Test
    fun `PROTO-4-b AdminChanged flat format parses userId and role`() {
        // AdminChanged: no schema, flat top-level fields (backward-compat)
        val json = """
            {
              "type": "AdminChanged",
              "userId": "user-abc",
              "role": "admin"
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to AdminChanged", msg is WsServerMessage.AdminChanged)
        val changed = msg as WsServerMessage.AdminChanged
        assertEquals("userId should be user-abc", "user-abc", changed.userId)
        assertEquals("role should be admin", "admin", changed.role)
    }

    @Test
    fun `PROTO-4-c RoomInfoUpdated flat format parses title and announcement`() {
        // RoomInfoUpdated: no schema, flat top-level fields (backward-compat)
        val json = """
            {
              "type": "RoomInfoUpdated",
              "title": "New Room Title",
              "announcement": "This is the announcement"
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to RoomInfoUpdated", msg is WsServerMessage.RoomInfoUpdated)
        val updated = msg as WsServerMessage.RoomInfoUpdated
        assertEquals("title should match", "New Room Title", updated.title)
        assertEquals("announcement should match", "This is the announcement", updated.announcement)
    }

    // ─── PROTO-5: Pong ────────────────────────────────────────────────────────

    @Test
    fun `PROTO-5 Pong message parses to Pong class not Unknown`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json
        val json = """
            {
              "type": "Pong",
              "msg_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cd",
              "timestamp": 1234567894
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Pong should NOT parse to Unknown", msg !is WsServerMessage.Unknown)
        assertTrue("Pong should parse to WsServerMessage.Pong", msg is WsServerMessage.Pong)
        val pong = msg as WsServerMessage.Pong
        assertEquals(
            "msgId should match",
            "6ba7b810-9dad-11d1-80b4-00c04fd430cd",
            pong.msgId
        )
        assertEquals("timestamp should be 1234567894", 1234567894L, pong.timestamp)
    }

    // ─── PROTO-6: Unknown ─────────────────────────────────────────────────────

    @Test
    fun `PROTO-6 unknown type FooBar parses to Unknown class with type field`() {
        val json = """{"type":"FooBar","someField":123}"""

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Unknown signal should parse to Unknown class", msg is WsServerMessage.Unknown)
        assertEquals("type should be FooBar", "FooBar", (msg as WsServerMessage.Unknown).type)
    }

    @Test
    fun `PROTO-6-b missing type field parses to Unknown with __missing_type__`() {
        val json = """{"foo":"bar","someField":123}"""

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Missing type should produce Unknown", msg is WsServerMessage.Unknown)
        assertEquals(
            "type should be __missing_type__",
            "__missing_type__",
            (msg as WsServerMessage.Unknown).type
        )
    }

    @Test
    fun `PROTO-6-c malformed JSON returns Unknown via exception fallback`() {
        // Invalid JSON should throw during Gson parse, not silently return null
        val json = """not-valid-json"""
        var threw = false
        try {
            gson.fromJson(json, WsServerMessage::class.java)
        } catch (e: Exception) {
            threw = true
        }
        assertTrue("Malformed JSON should throw exception (not silently ignored)", threw)
    }

    // ─── PROTO-7: RoomMessage payload-nested ─────────────────────────────────

    @Test
    fun `PROTO-7 RoomMessage payload nested fields parsed correctly`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/RoomMessage.schema.json
        val json = """
            {
              "type": "RoomMessage",
              "payload": {
                "msg_id": "msg-001",
                "user_id": "user-7",
                "nickname": "Alice",
                "content": "Hello world"
              },
              "timestamp": 1700000000000
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to RoomMessage", msg is WsServerMessage.RoomMessage)
        val rm = msg as WsServerMessage.RoomMessage
        assertEquals("payload.msgId should be msg-001", "msg-001", rm.payload.msgId)
        assertEquals("payload.userId should be user-7", "user-7", rm.payload.userId)
        assertEquals("payload.content should match", "Hello world", rm.payload.content)
    }

    // ─── PROTO-8: Result types with code field ────────────────────────────────

    @Test
    fun `PROTO-8-a SendGiftResult parses msgId and code`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/SendGiftResult.schema.json
        val json = """
            {
              "type": "SendGiftResult",
              "msg_id": "gift-result-001",
              "code": 0,
              "timestamp": 1234567895
            }
        """.trimIndent()

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to SendGiftResult", msg is WsServerMessage.SendGiftResult)
        val result = msg as WsServerMessage.SendGiftResult
        assertEquals("msgId should match", "gift-result-001", result.msgId)
        assertEquals("code should be 0", 0, result.code)
    }

    @Test
    fun `PROTO-8-b TakeMicResult parses code`() {
        // PROTO-BINDING: doc/protocol/schemas/ws/TakeMicResult.schema.json
        val json = """{"type":"TakeMicResult","msg_id":"take-001","code":0,"timestamp":123}"""

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to TakeMicResult", msg is WsServerMessage.TakeMicResult)
        assertEquals("code should be 0", 0, (msg as WsServerMessage.TakeMicResult).code)
    }

    // ─── PROTO-9: UserKicked flat backward-compat ────────────────────────────

    @Test
    fun `PROTO-9 UserKicked flat format parses reason and cooldownSec`() {
        // UserKicked: no schema file, keep flat for backward-compat
        val json = """{"type":"UserKicked","reason":"spam","cooldown_sec":600}"""

        val msg = gson.fromJson(json, WsServerMessage::class.java)

        assertTrue("Should parse to UserKicked", msg is WsServerMessage.UserKicked)
        val kicked = msg as WsServerMessage.UserKicked
        assertEquals("reason should be spam", "spam", kicked.reason)
        assertEquals("cooldownSec should be 600", 600, kicked.cooldownSec)
    }
}
