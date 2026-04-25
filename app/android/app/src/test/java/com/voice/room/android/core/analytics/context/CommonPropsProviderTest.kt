package com.voice.room.android.core.analytics.context

import com.google.gson.Gson
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import org.junit.Assert.*
import org.junit.Test

/**
 * CommonPropsProvider TDD 测试 - Review Round 1 修复（T-30035）
 *
 * R1 修复（缺陷 1/3）：公共字段已升级为 [com.voice.room.android.core.analytics.queue.EventQueueEntity] 独立列；
 * propertiesJson 仅承载业务属性。本测试验证：
 *   - 公共字段写入实体列（不在 propertiesJson 中）
 *   - 业务属性进入 propertiesJson
 *   - 业务侧不能覆盖公共字段（reservedKeys 拦截）
 */
class CommonPropsProviderTest {

    private val gson = Gson()

    // ── 公共字段写入实体列 ─────────────────────────────────────────────────

    @Test
    fun `enrich populates network_type column from networkTypeProvider`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN",
            networkTypeProvider = { "WIFI" }
        )

        val entity = provider.enrich("test_event", emptyMap(), "session-123")

        assertEquals("network_type 应写入实体列", "WIFI", entity.networkType)
    }

    @Test
    fun `enrich network_type reflects dynamic provider value`() {
        var networkType = "WIFI"
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN",
            networkTypeProvider = { networkType }
        )

        val entity1 = provider.enrich("event_1", emptyMap(), "s1")
        networkType = "MOBILE"
        val entity2 = provider.enrich("event_2", emptyMap(), "s2")

        assertEquals("第一次应为 WIFI", "WIFI", entity1.networkType)
        assertEquals("切换后应为 MOBILE", "MOBILE", entity2.networkType)
    }

    @Test
    fun `enrich populates all common fields as entity columns alongside business properties in json`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-xyz",
            appVersion = "2.0.0",
            osVersion = "13",
            locale = "ar-SA",
            networkTypeProvider = { "MOBILE" },
            userIdProvider = { "u-100" }
        )

        val entity = provider.enrich("hall_view", mapOf("room_id" to "r1"), "sess-abc")

        // 公共字段在实体列
        assertEquals("dev-xyz", entity.deviceId)
        assertEquals("2.0.0", entity.appVersion)
        assertEquals("13", entity.osVersion)
        assertEquals("ar-SA", entity.locale)
        assertEquals("MOBILE", entity.networkType)
        assertEquals("u-100", entity.userId)
        assertEquals("sess-abc", entity.sessionId)

        // 业务属性在 propertiesJson
        @Suppress("UNCHECKED_CAST")
        val props = gson.fromJson(entity.propertiesJson, Map::class.java) as Map<String, Any?>
        assertEquals("r1", props["room_id"])
        assertFalse("device_id 不应出现在 propertiesJson", props.containsKey("device_id"))
        assertFalse("app_version 不应出现在 propertiesJson", props.containsKey("app_version"))
        assertFalse("network_type 不应出现在 propertiesJson", props.containsKey("network_type"))
    }

    @Test
    fun `enrich without explicit networkTypeProvider uses default UNKNOWN`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN"
        )

        val entity = provider.enrich("test_event", emptyMap(), "session-001")

        assertNotNull("默认值时 network_type 不应为 null", entity.networkType)
        assertEquals("默认值应为 UNKNOWN", "UNKNOWN", entity.networkType)
    }

    @Test
    fun `enrich network_type NONE when no connectivity`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN",
            networkTypeProvider = { "NONE" }
        )

        val entity = provider.enrich("app_launch", emptyMap(), "s1")

        assertEquals("NONE", entity.networkType)
    }

    // ── R1 缺陷 3：业务层不得覆盖公共字段（零容忍红线 #7）────────────────

    @Test
    fun `enrich rejects business override of reserved common keys`() {
        val provider = CommonPropsProvider(
            deviceId = "real-device",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN",
            networkTypeProvider = { "WIFI" },
            userIdProvider = { "real-user" }
        )

        // 业务尝试通过 properties 伪造 device_id / session_id / app_version
        val entity = provider.enrich(
            "test_event",
            mapOf(
                "device_id" to "spoofed",
                "session_id" to "spoofed-session",
                "app_version" to "0.0.0",
                "user_id" to "spoofed-user",
                "biz_field" to "ok"
            ),
            sessionId = "real-session"
        )

        // 实体列必须保留权威值
        assertEquals("device_id 必须为权威值，不被业务覆盖", "real-device", entity.deviceId)
        assertEquals("session_id 必须为权威值", "real-session", entity.sessionId)
        assertEquals("app_version 必须为权威值", "1.0.0", entity.appVersion)
        assertEquals("user_id 必须为权威值", "real-user", entity.userId)

        // propertiesJson 应保留业务字段，但保留 key 已被丢弃
        @Suppress("UNCHECKED_CAST")
        val props = gson.fromJson(entity.propertiesJson, Map::class.java) as Map<String, Any?>
        assertEquals("ok", props["biz_field"])
        assertFalse("device_id 应被 reservedKeys 丢弃", props.containsKey("device_id"))
        assertFalse("session_id 应被 reservedKeys 丢弃", props.containsKey("session_id"))
        assertFalse("user_id 应被 reservedKeys 丢弃", props.containsKey("user_id"))
    }

    // ── R1 缺陷 6：scrubExtras 保留 value 类型 ────────────────────────────

    @Test
    fun `enrich preserves non-string types in propertiesJson`() {
        val provider = CommonPropsProvider(
            deviceId = "d",
            appVersion = "1",
            osVersion = "14",
            locale = "zh",
            filter = SensitiveFilter()
        )

        val entity = provider.enrich(
            "gift_send",
            mapOf(
                "amount" to 100,        // Int
                "ratio" to 0.5,         // Double
                "vip" to true,          // Boolean
                "ts" to 1720000000000L  // Long — 不应被误判为手机号
            ),
            "s1"
        )

        @Suppress("UNCHECKED_CAST")
        val props = gson.fromJson(entity.propertiesJson, Map::class.java) as Map<String, Any?>
        // Gson 反序列化时数字会变 Double，但关键是不能被脱敏成 "***"
        assertNotEquals("amount 不应被脱敏", "***", props["amount"])
        assertEquals(true, props["vip"])
        // ts 13 位毫秒时间戳：缺陷 9 修复后不应再被误判为手机号
        assertNotEquals("毫秒时间戳不应被误判为手机号", "***", props["ts"])
    }
}
