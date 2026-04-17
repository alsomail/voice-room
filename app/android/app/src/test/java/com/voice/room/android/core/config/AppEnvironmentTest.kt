package com.voice.room.android.core.config

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class AppEnvironmentTest {
    @Test
    fun `fromBuildConfig trims values and flags loopback hosts for physical-device debug`() {
        val environment = AppEnvironment.fromBuildConfig(
            environmentName = " dev ",
            apiBaseUrl = " http://127.0.0.1:3000/api/ ",
            wsUrl = " ws://localhost:3000/ws/ ",
            analyticsEndpoint = " https://analytics-dev.example.com/collect/ "
        )

        assertEquals("dev", environment.environmentName)
        assertEquals("http://127.0.0.1:3000/api", environment.apiBaseUrl)
        assertEquals("ws://localhost:3000/ws", environment.wsUrl)
        assertEquals("https://analytics-dev.example.com/collect", environment.analyticsEndpoint)
        assertTrue(
            environment.validateForPhysicalDevice().any {
                it.contains("loopback", ignoreCase = true)
            }
        )
    }

    @Test
    fun `validateForPhysicalDevice accepts LAN endpoints`() {
        val environment = AppEnvironment.fromBuildConfig(
            environmentName = "test",
            apiBaseUrl = "http://192.168.1.8:3000/api",
            wsUrl = "ws://192.168.1.8:3000/ws",
            analyticsEndpoint = "https://analytics-dev.example.com/collect"
        )

        assertTrue(environment.validateForPhysicalDevice().isEmpty())
    }
}
