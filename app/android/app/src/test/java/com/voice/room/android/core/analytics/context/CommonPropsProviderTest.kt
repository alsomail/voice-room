package com.voice.room.android.core.analytics.context

import com.google.gson.Gson
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import org.junit.Assert.*
import org.junit.Test

/**
 * CommonPropsProvider TDD 测试 - Review Round 1 修复（T-30035）
 *
 * MEDIUM-1 验收：enrich() 输出包含 network_type 字段
 */
class CommonPropsProviderTest {

    private val gson = Gson()

    // ── [RED] network_type 字段存在于 enrich 结果 ──────────────────────────

    @Test
    fun `enrich includes network_type from networkTypeProvider`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN",
            networkTypeProvider = { "WIFI" }
        )

        val entity = provider.enrich("test_event", emptyMap(), "session-123")

        val props = gson.fromJson(entity.propertiesJson, Map::class.java)
        assertEquals(
            "enrich 结果应包含 network_type=WIFI",
            "WIFI",
            props["network_type"]
        )
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

        val props1 = gson.fromJson(entity1.propertiesJson, Map::class.java)
        val props2 = gson.fromJson(entity2.propertiesJson, Map::class.java)

        assertEquals("第一次应为 WIFI", "WIFI", props1["network_type"])
        assertEquals("切换后应为 MOBILE", "MOBILE", props2["network_type"])
    }

    @Test
    fun `enrich still contains all existing common props alongside network_type`() {
        val provider = CommonPropsProvider(
            deviceId = "dev-xyz",
            appVersion = "2.0.0",
            osVersion = "13",
            locale = "ar-SA",
            networkTypeProvider = { "MOBILE" }
        )

        val entity = provider.enrich("hall_view", mapOf("room_id" to "r1"), "sess-abc")

        val props = gson.fromJson(entity.propertiesJson, Map::class.java)
        assertEquals("dev-xyz", props["device_id"])
        assertEquals("2.0.0", props["app_version"])
        assertEquals("13", props["os_version"])
        assertEquals("ar-SA", props["locale"])
        assertEquals("MOBILE", props["network_type"])
        assertEquals("r1", props["room_id"])
    }

    @Test
    fun `enrich without explicit networkTypeProvider uses default UNKNOWN`() {
        // 无 networkTypeProvider 参数时应有默认值，不抛异常
        val provider = CommonPropsProvider(
            deviceId = "dev-001",
            appVersion = "1.0.0",
            osVersion = "14",
            locale = "zh-CN"
            // networkTypeProvider 使用默认值
        )

        val entity = provider.enrich("test_event", emptyMap(), "session-001")

        val props = gson.fromJson(entity.propertiesJson, Map::class.java)
        assertNotNull("默认值时 network_type 不应为 null", props["network_type"])
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

        val props = gson.fromJson(entity.propertiesJson, Map::class.java)
        assertEquals("无网络时应为 NONE", "NONE", props["network_type"])
    }
}
