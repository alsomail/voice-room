package com.voice.room.android.core.config

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * U2.2 — staging flavor BuildConfig 字段冻结值（src/testStaging/ 仅在 staging flavor 下编译执行）
 */
class BuildConfigStagingFlavorTest {

    @Test
    fun U2_2_staging_apiBaseUrl() {
        assertEquals("https://stg-api.example.com/api", BuildConfig.API_BASE_URL)
    }

    @Test
    fun U2_2_staging_wsUrl_isWss() {
        assertEquals("wss://stg-api.example.com/ws", BuildConfig.WS_URL)
    }

    @Test
    fun U2_2_staging_analyticsEndpoint() {
        assertEquals("https://stg-api.example.com/api/v1/events/batch", BuildConfig.ANALYTICS_ENDPOINT)
    }

    @Test
    fun U2_2_staging_appEnvironment() {
        assertEquals("staging", BuildConfig.APP_ENVIRONMENT)
    }
}
