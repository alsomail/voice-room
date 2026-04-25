package com.voice.room.android.core.analytics.context

import com.google.gson.Gson
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import com.voice.room.android.core.analytics.queue.EventQueueEntity

/**
 * 公共属性注入器（T-30035）
 *
 * 将 device_id / app_version / os_version / locale / network_type 等公共字段
 * 升级为 [EventQueueEntity] 的独立列；同时入队时刻抓取 user_id / network_type，
 * 避免 flush 时漂移。
 *
 * R1 修复（缺陷 1）：公共字段不再混入 propertiesJson，而是作为独立列与服务端 `EventInput` 对齐。
 *
 * R1 修复（缺陷 3）：propertiesJson 仅承载 **业务属性**，不含公共字段；
 * 即便业务侧传入同名 key（如 `device_id`），也不会污染公共列 — 公共列以 [enrich] 参数为权威。
 *
 * @param deviceId            设备唯一标识（安装时生成，DataStore 持久化）
 * @param appVersion          App 版本号（BuildConfig.VERSION_NAME）
 * @param osVersion           Android 版本（Build.VERSION.RELEASE）
 * @param locale              语言地区（Locale.getDefault().toString()）
 * @param networkTypeProvider 网络类型提供者（ConnectivityManager 获取 WIFI/MOBILE/NONE 等）
 * @param userIdProvider      当前登录用户 ID 提供者（未登录返回 null）
 * @param filter              脱敏过滤器
 */
class CommonPropsProvider(
    private val deviceId: String,
    private val appVersion: String,
    private val osVersion: String,
    private val locale: String,
    private val networkTypeProvider: () -> String = { "UNKNOWN" },
    private val userIdProvider: () -> String? = { null },
    private val filter: SensitiveFilter = SensitiveFilter(),
    private val gson: Gson = Gson()
) {

    /**
     * 将公共属性注入事件，返回可入队的 [EventQueueEntity]。
     *
     * propertiesJson 仅含 **业务属性**（已脱敏）；公共字段写入 entity 的独立列。
     * 业务层若传入与公共字段同名的 key（如 `device_id`），将被 [reservedKeys] 丢弃并打印 warn。
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
        // 1) 丢弃业务侧对公共字段的覆盖企图（缺陷 3 零容忍红线 #7）
        val sanitized = properties.filterKeys { key ->
            val isReserved = reservedKeys.contains(key)
            if (isReserved) {
                android.util.Log.w(
                    "CommonPropsProvider",
                    "Business event '$eventName' attempted to override reserved common field '$key' — dropped."
                )
            }
            !isReserved
        }

        // 2) 脱敏业务属性（保留类型，缺陷 6）
        val scrubbed = filter.scrubExtras(sanitized)

        // 3) 序列化 — 仅业务属性
        val propertiesJson = gson.toJson(scrubbed)

        return EventQueueEntity(
            eventName = eventName,
            deviceId = deviceId,
            userId = userIdProvider(),
            sessionId = sessionId,
            clientTs = System.currentTimeMillis(),
            appVersion = appVersion,
            osVersion = osVersion,
            locale = locale,
            networkType = networkTypeProvider(),
            propertiesJson = propertiesJson
        )
    }

    companion object {
        /**
         * 公共字段保留 key 集合 — 业务层不得通过 properties 覆盖。
         * 与服务端 `EventInput` 顶层字段一一对应。
         */
        val reservedKeys: Set<String> = setOf(
            "device_id",
            "user_id",
            "session_id",
            "client_ts",
            "app_version",
            "os_version",
            "locale",
            "network_type",
            "event_name"
        )
    }
}
