package com.voice.room.android.core.config

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * U2.3 — prod flavor BuildConfig 字段冻结值（src/testProd/ 仅在 prod flavor 下编译执行）
 */
class BuildConfigProdFlavorTest {

    @Test
    fun U2_3_prod_apiBaseUrl() {
        assertEquals("https://api.example.com/api", BuildConfig.API_BASE_URL)
    }

    @Test
    fun U2_3_prod_wsUrl_isWss() {
        assertEquals("wss://api.example.com/ws", BuildConfig.WS_URL)
    }

    @Test
    fun U2_3_prod_analyticsEndpoint() {
        assertEquals("https://api.example.com/api/v1/events/batch", BuildConfig.ANALYTICS_ENDPOINT)
    }

    @Test
    fun U2_3_prod_appEnvironment() {
        assertEquals("prod", BuildConfig.APP_ENVIRONMENT)
    }
}
