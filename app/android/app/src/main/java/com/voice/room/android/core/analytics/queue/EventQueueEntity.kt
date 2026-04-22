package com.voice.room.android.core.analytics.queue

/**
 * 事件队列实体（T-30035）
 *
 * 对应数据库表 event_queue，每行代表一个待上报的埋点事件。
 * 故意不添加 Room @Entity 注解以避免 kapt 依赖（MVP 阶段）；
 * 添加 Room 依赖后可直接加注解激活。
 *
 * @param id          主键（自增）
 * @param eventName   事件名（如 "login_success"）
 * @param propertiesJson 事件属性 JSON 字符串（脱敏后）
 * @param sessionId   所属 session UUID
 * @param clientTs    客户端时间戳（毫秒）
 * @param createdAt   入队时间戳（毫秒），用于淘汰策略
 */
data class EventQueueEntity(
    val id: Long = 0L,
    val eventName: String,
    val propertiesJson: String,
    val sessionId: String,
    val clientTs: Long,
    val createdAt: Long = System.currentTimeMillis()
)
