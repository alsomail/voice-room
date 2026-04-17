package com.voice.room.android.core.config

interface IRemoteConfigService {
    fun getBoolean(key: String, defaultValue: Boolean): Boolean
}
