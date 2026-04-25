package com.voice.room.android.core.analytics.queue

/**
 * 事件队列实体（T-30035）
 *
 * 对应数据库表 event_queue，每行代表一个待上报的埋点事件。
 * 故意不添加 Room @Entity 注解以避免 kapt 依赖（MVP 阶段）；
 * 添加 Room 依赖后可直接加注解激活。
 *
 * R1 修复（缺陷 1）：公共字段升级为独立列，与服务端 `EventInput`（writer.rs）一一对应，
 * 入队时刻抓取一次，避免 flush 时 session/network 漂移。
 *
 * @param id          主键（自增）
 * @param eventName   事件名（如 "login_success"）
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
data class EventQueueEntity(
    val id: Long = 0L,
    val eventName: String,
    val deviceId: String,
    val userId: String? = null,
    val sessionId: String,
    val clientTs: Long,
    val appVersion: String,
    val osVersion: String,
    val locale: String,
    val networkType: String,
    val propertiesJson: String,
    val createdAt: Long = System.currentTimeMillis()
)
