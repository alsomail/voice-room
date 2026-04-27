package com.voice.room.android.core.config

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — T-30050 productFlavors BuildConfig 字段冻结
 *
 * 本 test sourceset 在 default flavor (local) 下执行，对应字段冻结表 §2.3：
 *   API_BASE_URL          = http://10.0.2.2:3000/api
 *   WS_URL                = ws://10.0.2.2:3000/ws
 *   ANALYTICS_ENDPOINT    = http://10.0.2.2:3000/api/v1/events/batch
 *   APP_ENVIRONMENT       = local
 *
 * 用例编号：U2.1（local flavor BuildConfig 字段值）
 *          U5.2（默认 flavor 单测 0 回归 — 现有 testDebugUnitTest 等价 testLocalDebugUnitTest）
 */
class BuildConfigFlavorTest {

    @Test
    fun U2_1_local_apiBaseUrl_pointsToEmulatorHost() {
        assertEquals(
            "local flavor 的 API_BASE_URL 必须指向模拟器→宿主映射地址",
            "http://10.0.2.2:3000/api",
            BuildConfig.API_BASE_URL
        )
    }

    @Test
    fun U2_1_local_wsUrl_isCleartextWs() {
        assertEquals(
            "local flavor 的 WS_URL 必须为 ws://（非加密，与 AppServer dev 对称）",
            "ws://10.0.2.2:3000/ws",
            BuildConfig.WS_URL
        )
    }

    @Test
    fun U2_1_local_analyticsEndpoint_isDerivedFromApiBaseUrl() {
        assertEquals(
            "local flavor 的 ANALYTICS_ENDPOINT 必须指向 /api/v1/events/batch",
            "http://10.0.2.2:3000/api/v1/events/batch",
            BuildConfig.ANALYTICS_ENDPOINT
        )
    }

    @Test
    fun U2_1_local_appEnvironment_isLowercaseLocal() {
        assertEquals(
            "APP_ENVIRONMENT 必须为小写 'local'，与 .env.{profile}.example 命名对齐",
            "local",
            BuildConfig.APP_ENVIRONMENT
        )
    }

    @Test
    fun U2_1_buildConfig_allFlavorFields_existAndNonBlank() {
        // R3 防御：flavor 字段 typo 时回落 defaultConfig 兜底，但字段必须存在
        assertNotNull(BuildConfig.API_BASE_URL)
        assertNotNull(BuildConfig.WS_URL)
        assertNotNull(BuildConfig.ANALYTICS_ENDPOINT)
        assertNotNull(BuildConfig.APP_ENVIRONMENT)
        assertTrue(BuildConfig.API_BASE_URL.isNotBlank())
        assertTrue(BuildConfig.WS_URL.isNotBlank())
        assertTrue(BuildConfig.ANALYTICS_ENDPOINT.isNotBlank())
        assertTrue(BuildConfig.APP_ENVIRONMENT.isNotBlank())
    }
}
