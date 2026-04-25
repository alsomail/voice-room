package com.voice.room.android.core.analytics.queue

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [RoomEventQueueRepository] 单元测试（T-30035 / R1 批 2 缺陷 7）
 *
 * 不依赖真实 Room / Android 运行时：通过手写 [FakeEventQueueRoomDao] 模拟
 * Room 生成的 `@Dao` 实现，验证 LRU cap 容量淘汰逻辑。
 */
class RoomEventQueueRepositoryTest {

    private fun sample(id: Long = 0L, name: String = "evt"): EventQueueEntity =
        EventQueueEntity(
            id = id,
            eventName = name,
            deviceId = "d",
            userId = null,
            sessionId = "s",
            clientTs = 1L,
            appVersion = "1",
            osVersion = "Android 14",
            locale = "en-US",
            networkType = "WIFI",
            propertiesJson = "{}",
            createdAt = id,
        )

    @Test
    fun `insert below cap does not trigger eviction`() = runTest {
        val fake = FakeEventQueueRoomDao()
        val repo = RoomEventQueueRepository(fake, maxCapacity = 1000)

        repeat(500) { repo.insert(sample(id = it.toLong())) }

        assertEquals(500, repo.size())
        assertEquals(0, fake.deleteOldestCalls.sum())
    }

    @Test
    fun `insert above cap triggers deleteOldest by overflow count`() = runTest {
        val fake = FakeEventQueueRoomDao()
        val repo = RoomEventQueueRepository(fake, maxCapacity = 10)

        repeat(11) { repo.insert(sample(id = it.toLong())) }

        // 第 11 次 insert 时 count=11 → delete 1
        assertEquals(1, fake.deleteOldestCalls.sum())
        assertEquals(10, repo.size())
    }

    @Test
    fun `insert massively above cap evicts down to cap`() = runTest {
        val fake = FakeEventQueueRoomDao()
        val repo = RoomEventQueueRepository(fake, maxCapacity = 5)

        repeat(20) { repo.insert(sample(id = it.toLong())) }

        assertEquals(5, repo.size())
        assertTrue("总淘汰应 = 15", fake.deleteOldestCalls.sum() == 15)
    }

    @Test
    fun `getOldest deleteByIds clear delegate to dao`() = runTest {
        val fake = FakeEventQueueRoomDao()
        val repo = RoomEventQueueRepository(fake)

        repeat(3) { repo.insert(sample(id = it.toLong(), name = "e$it")) }
        val oldest = repo.getOldest(2)
        assertEquals(2, oldest.size)

        // Fake 给每条赋自增 id 1..3
        repo.deleteByIds(listOf(1L, 2L))
        assertEquals(1, repo.size())

        repo.clear()
        assertEquals(0, repo.size())
    }

    /** 简单的内存 Fake，模拟 Room 生成的 [EventQueueRoomDao] 实现 */
    private class FakeEventQueueRoomDao : EventQueueRoomDao {
        private val rows = mutableListOf<EventQueueEntity>()
        private var auto = 1L
        val deleteOldestCalls = mutableListOf<Int>()

        override suspend fun insert(entity: EventQueueEntity): Long {
            val id = auto++
            val created = if (entity.createdAt > 0) entity.createdAt else id
            rows += entity.copy(id = id, createdAt = created)
            return id
        }

        override suspend fun count(): Int = rows.size

        override suspend fun getOldest(limit: Int): List<EventQueueEntity> =
            rows.sortedWith(compareBy({ it.createdAt }, { it.id })).take(limit)

        override suspend fun deleteByIds(ids: List<Long>) {
            rows.removeAll { it.id in ids }
        }

        override suspend fun deleteOldest(count: Int) {
            deleteOldestCalls += count
            val toDrop = rows.sortedWith(compareBy({ it.createdAt }, { it.id }))
                .take(count)
                .map { it.id }
            rows.removeAll { it.id in toDrop }
        }

        override suspend fun clear() {
            rows.clear()
        }
    }
}
