package com.voice.room.android.core.analytics.queue

/**
 * 事件队列 DAO 接口（T-30035）
 *
 * 定义事件队列的存储操作。MVP 阶段由 [InMemoryEventQueueDao] 实现（测试/dev），
 * 生产阶段添加 Room 依赖后用 Room 实现替换。
 *
 * 队列策略：
 * - 容量上限 1000 条；超过时由 [insert] 自动淘汰最旧条目
 * - flush 时取最多 100 条发送
 */
interface EventQueueDao {

    /** 插入一条事件；若队列超过 [MAX_CAPACITY] 则自动删除最旧的一条 */
    suspend fun insert(entity: EventQueueEntity)

    /** 返回当前队列总条数 */
    suspend fun size(): Int

    /**
     * 按 created_at 升序取最旧的 [limit] 条（待 flush 批次）
     */
    suspend fun getOldest(limit: Int = 100): List<EventQueueEntity>

    /** 按 id 列表删除（成功上报后调用）*/
    suspend fun deleteByIds(ids: List<Long>)

    /** 删除最旧的 [count] 条（淘汰超出容量的条目）*/
    suspend fun deleteOldest(count: Int)

    /** 清空所有事件（仅用于测试） */
    suspend fun clear()

    companion object {
        const val MAX_CAPACITY = 1000
        const val FLUSH_BATCH_SIZE = 100
    }
}
