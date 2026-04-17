package com.voice.room.android.core.config

class InMemoryRemoteConfigService(
    private val values: Map<String, Boolean> = emptyMap()
) : IRemoteConfigService {
    override fun getBoolean(key: String, defaultValue: Boolean): Boolean = values[key] ?: defaultValue
}
