package com.voice.room.android.core.config

import java.net.URI

data class AppEnvironment(
    val environmentName: String,
    val apiBaseUrl: String,
    val wsUrl: String,
    val analyticsEndpoint: String
) {
    fun validateForPhysicalDevice(): List<String> {
        val warnings = mutableListOf<String>()

        if (apiBaseUrl.isLoopbackHost() || wsUrl.isLoopbackHost()) {
            warnings += "Loopback hosts are not reachable from a physical-device debug session."
        }

        return warnings
    }

    companion object {
        fun fromBuildConfig(
            environmentName: String,
            apiBaseUrl: String,
            wsUrl: String,
            analyticsEndpoint: String
        ): AppEnvironment {
            return AppEnvironment(
                environmentName = environmentName.trim(),
                apiBaseUrl = apiBaseUrl.trim().trimEnd('/'),
                wsUrl = wsUrl.trim().trimEnd('/'),
                analyticsEndpoint = analyticsEndpoint.trim().trimEnd('/')
            )
        }
    }
}

private fun String.isLoopbackHost(): Boolean {
    val host = runCatching { URI(this).host?.lowercase() }.getOrNull() ?: return false

    return host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" || host == "::1"
}
