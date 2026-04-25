package com.voice.room.android.core.analytics.queue

/**
 * Room-backed [EventQueueDao] 实现（T-30035 / R1 批 2 缺陷 7）
 *
 * 适配器：把抽象 [EventQueueDao] 接口的语义映射到 Room 原生 [EventQueueRoomDao]。
 *
 * **LRU 容量上限**：[EventQueueDao.MAX_CAPACITY] = 1000；超出时由 [insert] 删除最旧的若干条
 * （不依赖触发器，避免不同 SQLite 编译选项的兼容性问题）。
 *
 * 此实现可在生产构建中替换 [InMemoryEventQueueDao]，进程被杀后未上报事件不丢失，
 * 满足 TDS T-30035 §队列策略的 Room 持久化要求。
 *
 * @param raw Room 生成的原始 @Dao（由 AppDatabase 提供）
 */
class RoomEventQueueRepository(
    private val raw: EventQueueRoomDao,
    private val maxCapacity: Int = EventQueueDao.MAX_CAPACITY
) : EventQueueDao {

    override suspend fun insert(entity: EventQueueEntity) {
        raw.insert(entity)
        val current = raw.count()
        if (current > maxCapacity) {
            raw.deleteOldest(current - maxCapacity)
        }
    }

    override suspend fun size(): Int = raw.count()

    override suspend fun getOldest(limit: Int): List<EventQueueEntity> = raw.getOldest(limit)

    override suspend fun deleteByIds(ids: List<Long>) {
        if (ids.isEmpty()) return
        raw.deleteByIds(ids)
    }

    override suspend fun deleteOldest(count: Int) {
        if (count <= 0) return
        raw.deleteOldest(count)
    }

    override suspend fun clear() = raw.clear()
}
