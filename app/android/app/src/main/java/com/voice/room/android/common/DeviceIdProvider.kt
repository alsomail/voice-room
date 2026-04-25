package com.voice.room.android.common

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import java.util.UUID

/**
 * 设备唯一标识持久化（T-30035 / R1 批 2 缺陷 2）
 *
 * 在首次启动时生成 UUID，DataStore Preferences 持久化。
 * 此值作为 Analytics 公共字段 `device_id` 的来源；
 * 卸载重装会重置（符合数据保留期治理预期）。
 *
 * 注意：必须在 IO 线程 / 协程中调用 [getOrCreate]；
 * 为简化 AppContainer.fromBuildConfig 的同步组装，提供 [getOrCreateBlocking] 兜底，
 * 仅在 Application.onCreate 中调用一次（启动闪屏期可接受短暂 IO）。
 */
object DeviceIdProvider {

    private const val PREFS_NAME = "voice_room_device"
    private val KEY_DEVICE_ID = stringPreferencesKey("device_id")

    private val Context.deviceDataStore: DataStore<Preferences> by preferencesDataStore(
        name = PREFS_NAME
    )

    suspend fun getOrCreate(context: Context): String {
        val store = context.applicationContext.deviceDataStore
        val existing = store.data.first()[KEY_DEVICE_ID]
        if (!existing.isNullOrBlank()) return existing
        val newId = UUID.randomUUID().toString()
        store.edit { prefs -> prefs[KEY_DEVICE_ID] = newId }
        return newId
    }

    /** 仅供 Application.onCreate 调用一次的同步入口 */
    fun getOrCreateBlocking(context: Context): String =
        runBlocking { getOrCreate(context) }
}
