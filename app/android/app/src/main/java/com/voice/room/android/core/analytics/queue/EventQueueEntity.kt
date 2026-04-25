package com.voice.room.android.core.analytics.queue

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * 事件队列实体（T-30035）
 *
 * 对应 Room 数据库表 `event_queue`，每行代表一个待上报的埋点事件。
 *
 * R1 修复（缺陷 1）：公共字段升级为独立列，与服务端 `EventInput`（writer.rs）一一对应，
 * 入队时刻抓取一次，避免 flush 时 session/network 漂移。
 *
 * R1 批 2 修复（缺陷 7）：增加 Room `@Entity` 注解 + `@PrimaryKey(autoGenerate = true)`；
 * 表名 `event_queue` 与 [com.voice.room.android.core.analytics.EventReportClient.TABLE_NAME] 一致；
 * 在 [com.voice.room.android.core.storage.AppDatabase] 中注册。`createdAt` 列建立索引便于
 * `ORDER BY created_at ASC LIMIT N`（getOldest / deleteOldest 热路径）。
 *
 * @param id          主键（自增）
 * @param eventName   事件名（如 "login_verify_success"）
 * @param deviceId    设备 ID（公共字段，必填，入队时刻抓取）
 * @param userId      用户 ID（可选，未登录时为 null）
 * @param sessionId   所属 session UUID
 * @param clientTs    客户端时间戳（毫秒）
 * @param appVersion  App 版本（公共字段，入队时刻抓取）
 * @param osVersion   OS 版本（公共字段，入队时刻抓取）
 * @param locale      语言地区（公共字段，入队时刻抓取）
 * @param networkType 网络类型 WIFI/MOBILE/NONE（公共字段，入队时刻抓取）
 * @param propertiesJson 业务属性 JSON 字符串（脱敏后；为对象不为标量字符串）
 * @param createdAt   入队时间戳（毫秒），用于淘汰策略
 */
@Entity(tableName = "event_queue")
data class EventQueueEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0L,
    @ColumnInfo(name = "event_name")
    val eventName: String,
    @ColumnInfo(name = "device_id")
    val deviceId: String,
    @ColumnInfo(name = "user_id")
    val userId: String? = null,
    @ColumnInfo(name = "session_id")
    val sessionId: String,
    @ColumnInfo(name = "client_ts")
    val clientTs: Long,
    @ColumnInfo(name = "app_version")
    val appVersion: String,
    @ColumnInfo(name = "os_version")
    val osVersion: String,
    @ColumnInfo(name = "locale")
    val locale: String,
    @ColumnInfo(name = "network_type")
    val networkType: String,
    @ColumnInfo(name = "properties_json")
    val propertiesJson: String,
    @ColumnInfo(name = "created_at", index = true)
    val createdAt: Long = System.currentTimeMillis()
)
