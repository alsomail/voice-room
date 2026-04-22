package com.voice.room.android.core.analytics.context

import com.google.gson.Gson
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import com.voice.room.android.core.analytics.queue.EventQueueEntity

/**
 * 公共属性注入器（T-30035）
 *
 * 将 device_id / app_version / os_version / locale / network_type
 * 自动附加到每个事件属性中。
 *
 * 在 MVP 阶段：device_id 使用随机 UUID（DataStore 持久化留待后续迭代）。
 *
 * @param deviceId            设备唯一标识（安装时生成，DataStore 持久化）
 * @param appVersion          App 版本号（BuildConfig.VERSION_NAME）
 * @param osVersion           Android 版本（Build.VERSION.RELEASE）
 * @param locale              语言地区（Locale.getDefault().toString()）
 * @param networkTypeProvider 网络类型提供者（ConnectivityManager 获取 WIFI/MOBILE/NONE 等）
 * @param filter              脱敏过滤器
 */
class CommonPropsProvider(
    private val deviceId: String,
    private val appVersion: String,
    private val osVersion: String,
    private val locale: String,
    private val networkTypeProvider: () -> String = { "UNKNOWN" },
    private val filter: SensitiveFilter = SensitiveFilter(),
    private val gson: Gson = Gson()
) {

    /**
     * 将公共属性注入事件，返回可入队的 [EventQueueEntity]
     *
     * @param eventName  事件名
     * @param properties 业务层传入的事件属性（可含敏感数据，会被脱敏）
     * @param sessionId  当前 session UUID
     */
    fun enrich(
        eventName: String,
        properties: Map<String, Any?>,
        sessionId: String
    ): EventQueueEntity {
        // 先脱敏业务属性
        val scrubbed = filter.scrubExtras(properties)

        // 合并公共属性
        val merged = mutableMapOf<String, Any?>(
            "device_id" to deviceId,
            "app_version" to appVersion,
            "os_version" to osVersion,
            "locale" to locale,
            "network_type" to networkTypeProvider()
        )
        merged.putAll(scrubbed)

        val propertiesJson = gson.toJson(merged)
        return EventQueueEntity(
            eventName = eventName,
            propertiesJson = propertiesJson,
            sessionId = sessionId,
            clientTs = System.currentTimeMillis()
        )
    }
}
