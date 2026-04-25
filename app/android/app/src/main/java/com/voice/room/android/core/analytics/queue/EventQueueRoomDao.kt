package com.voice.room.android.core.analytics.queue

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query

/**
 * Room 原生 @Dao 接口（T-30035 / R1 批 2 缺陷 7）
 *
 * 仅由 [RoomEventQueueRepository] 调用；业务层不应直接依赖此接口。
 *
 * 容量淘汰（LRU cap=1000）由 [RoomEventQueueRepository.insert] 在事务外协调，
 * 此接口仅暴露原子写入与最旧条目删除两个原语。
 */
@Dao
interface EventQueueRoomDao {

    /**
     * 插入一条事件，返回自增主键。
     */
    @Insert
    suspend fun insert(entity: EventQueueEntity): Long

    /**
     * 当前队列总条数。
     */
    @Query("SELECT COUNT(*) FROM event_queue")
    suspend fun count(): Int

    /**
     * 取最旧 [limit] 条（按 created_at 升序）。
     */
    @Query("SELECT * FROM event_queue ORDER BY created_at ASC, id ASC LIMIT :limit")
    suspend fun getOldest(limit: Int): List<EventQueueEntity>

    /**
     * 按 id 列表删除。
     */
    @Query("DELETE FROM event_queue WHERE id IN (:ids)")
    suspend fun deleteByIds(ids: List<Long>)

    /**
     * 删除最旧的 [count] 条（淘汰超出容量的条目）。
     *
     * 实现：先 SELECT 出最旧 N 条 id，再 DELETE WHERE id IN ...，
     * 与 LIMIT 直接 DELETE 相比更兼容（部分 SQLite 编译时未启用 ENABLE_UPDATE_DELETE_LIMIT）。
     */
    @Query(
        "DELETE FROM event_queue WHERE id IN " +
            "(SELECT id FROM event_queue ORDER BY created_at ASC, id ASC LIMIT :count)"
    )
    suspend fun deleteOldest(count: Int)

    /**
     * 清空所有事件（仅测试或重置场景）。
     */
    @Query("DELETE FROM event_queue")
    suspend fun clear()
}
