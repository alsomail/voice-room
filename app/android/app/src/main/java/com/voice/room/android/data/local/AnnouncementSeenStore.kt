package com.voice.room.android.data.local

/**
 * 公告已读记录存储接口（T-30043）
 *
 * 记录用户在某房间最后一次看到公告的时间戳（毫秒）。
 * 生产环境可用 DataStore 实现持久化；测试环境注入 [InMemoryAnnouncementSeenStore]。
 */
interface AnnouncementSeenStore {

    /**
     * 获取指定房间的最后弹窗时间戳。
     *
     * @param roomId 目标房间 ID
     * @return 最后弹窗时间戳（毫秒）；若无记录则返回 null
     */
    fun get(roomId: String): Long?

    /**
     * 保存指定房间的弹窗时间戳。
     *
     * @param roomId      目标房间 ID
     * @param timestampMs 弹窗时间戳（epoch 毫秒），通常为 now
     */
    fun save(roomId: String, timestampMs: Long)
}

/**
 * 基于内存 Map 的 [AnnouncementSeenStore] 实现（T-30043）。
 *
 * 用于：
 * - 单元测试的 Fake 注入
 * - 生产环境的简化实现（进程重启后丢失，符合 MVP 要求）
 */
class InMemoryAnnouncementSeenStore : AnnouncementSeenStore {

    private val map = mutableMapOf<String, Long>()

    override fun get(roomId: String): Long? = map[roomId]

    override fun save(roomId: String, timestampMs: Long) {
        map[roomId] = timestampMs
    }
}
