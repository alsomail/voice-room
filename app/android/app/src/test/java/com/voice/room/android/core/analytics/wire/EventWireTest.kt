package com.voice.room.android.core.analytics.wire

import com.google.gson.Gson
import com.voice.room.android.core.analytics.queue.EventQueueEntity
import org.junit.Assert.*
import org.junit.Test

/**
 * EventWire 契约测试 — 严格对齐服务端 `app/server/src/core/analytics/writer.rs::EventInput`
 * （T-30035 R1 缺陷 1 修复）
 */
class EventWireTest {

    private val gson = Gson()

    private fun sampleEntity(id: Long = 1L, eventName: String = "login_success"): EventQueueEntity =
        EventQueueEntity(
            id = id,
            eventName = eventName,
            deviceId = "device-001",
            userId = "user-100",
            sessionId = "sess-abc",
            clientTs = 1720000000000L,
            appVersion = "1.2.0",
            osVersion = "Android 14",
            locale = "ar-SA",
            networkType = "WIFI",
            propertiesJson = """{"gift_id":"g1","amount":100}"""
        )

    @Test
    fun `toEventMap mirrors server EventInput schema`() {
        val map = EventWire.toEventMap(sampleEntity(), gson)

        assertEquals("login_success", map["event_name"])
        assertEquals("device-001", map["device_id"])
        assertEquals("user-100", map["user_id"])
        assertEquals("sess-abc", map["session_id"])
        assertEquals(1720000000000L, map["client_ts"])
        assertEquals("1.2.0", map["app_version"])
        assertEquals("Android 14", map["os_version"])
        assertEquals("ar-SA", map["locale"])
        assertEquals("WIFI", map["network_type"])

        // properties 必须是对象（Map），不能是字符串标量
        val props = map["properties"]
        assertTrue("properties 必须是 Map，不能是 String", props is Map<*, *>)
        @Suppress("UNCHECKED_CAST")
        val propsMap = props as Map<String, Any?>
        assertEquals("g1", propsMap["gift_id"])
    }

    @Test
    fun `toEventMap empty propertiesJson yields empty object not string`() {
        val map = EventWire.toEventMap(sampleEntity().copy(propertiesJson = ""), gson)
        val props = map["properties"]
        assertTrue("空 propertiesJson 应为对象", props is Map<*, *>)
        assertTrue((props as Map<*, *>).isEmpty())
    }

    @Test
    fun `toEventMap malformed json falls back to empty object never string`() {
        val map = EventWire.toEventMap(
            sampleEntity().copy(propertiesJson = "not-a-json"),
            gson
        )
        val props = map["properties"]
        assertTrue("非法 JSON 仍应降级为对象，不能写成 String 标量", props is Map<*, *>)
    }

    @Test
    fun `toHttpBody wraps batch under events key matching batch_events handler`() {
        val body = EventWire.toHttpBody(listOf(sampleEntity(1), sampleEntity(2, "gift_send")), gson)
        val events = body["events"] as List<*>
        assertEquals(2, events.size)
        val first = events[0] as Map<*, *>
        assertEquals("device-001", first["device_id"])
    }

    @Test
    fun `toWsEnvelope matches protocol section 6_3 with msg_id and payload events`() {
        val envelope = EventWire.toWsEnvelope(
            listOf(sampleEntity()),
            msgId = "msg-uuid-1",
            gson = gson
        )
        assertEquals("ReportEvent", envelope["type"])
        assertEquals("msg-uuid-1", envelope["msg_id"])

        val payload = envelope["payload"] as Map<*, *>
        val events = payload["events"] as List<*>
        assertEquals(1, events.size)
    }

    @Test
    fun `toWsEnvelope full json round trip matches server expectations`() {
        val envelope = EventWire.toWsEnvelope(
            listOf(sampleEntity()),
            msgId = "m1",
            gson = gson
        )
        val json = gson.toJson(envelope)
        assertTrue("应包含 msg_id 字段", json.contains("\"msg_id\""))
        assertTrue("payload 应为对象 (含 events 子键)", json.contains("\"payload\":{"))
        assertTrue("应包含 events 子键", json.contains("\"events\":["))
        assertTrue("properties 必须为对象", json.contains("\"properties\":{"))
        assertFalse("payload 不应是数组形式", json.contains("\"payload\":["))
    }
}
