package com.voice.room.android.core.analytics.queue

import androidx.annotation.VisibleForTesting
import java.util.concurrent.atomic.AtomicLong

/**
 * 内存事件队列 DAO 实现（T-30035）
 *
 * R1 批 2（缺陷 7）：仅供 JVM 单元测试使用；生产路径已切换到
 * [com.voice.room.android.core.analytics.queue.RoomEventQueueRepository]。
 */
@VisibleForTesting
class InMemoryEventQueueDao : EventQueueDao {

    private val lock = Any()
    private val queue = mutableListOf<EventQueueEntity>()
    private val idGen = AtomicLong(1L)

    override suspend fun insert(entity: EventQueueEntity) {
        synchronized(lock) {
            val withId = entity.copy(id = idGen.getAndIncrement())
            queue.add(withId)
            // 超容量时淘汰最旧
            while (queue.size > EventQueueDao.MAX_CAPACITY) {
                queue.removeAt(0)
            }
        }
    }

    override suspend fun size(): Int = synchronized(lock) { queue.size }

    override suspend fun getOldest(limit: Int): List<EventQueueEntity> =
        synchronized(lock) {
            queue.sortedBy { it.createdAt }.take(limit).toList()
        }

    override suspend fun deleteByIds(ids: List<Long>) {
        synchronized(lock) {
            val idSet = ids.toSet()
            queue.removeAll { it.id in idSet }
        }
    }

    override suspend fun deleteOldest(count: Int) {
        synchronized(lock) {
            val sorted = queue.sortedBy { it.createdAt }
            val toRemove = sorted.take(count)
            queue.removeAll(toRemove.toSet())
        }
    }

    override suspend fun clear() {
        synchronized(lock) { queue.clear() }
    }
}
