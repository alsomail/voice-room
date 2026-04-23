package com.voice.room.android.data.local

/**
 * [AnnouncementSeenStore] 的测试 Fake 实现（T-30043）
 *
 * 对 [InMemoryAnnouncementSeenStore] 的简单包装，同时暴露 [savedEntries] 供测试断言使用。
 */
class FakeAnnouncementSeenStore : AnnouncementSeenStore {

    /** 已写入的条目，key=roomId, value=timestampMs，可供测试检验 */
    val savedEntries = mutableMapOf<String, Long>()

    override fun get(roomId: String): Long? = savedEntries[roomId]

    override fun save(roomId: String, timestampMs: Long) {
        savedEntries[roomId] = timestampMs
    }
}
