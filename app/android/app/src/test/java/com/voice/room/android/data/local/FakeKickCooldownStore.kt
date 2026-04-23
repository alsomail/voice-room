package com.voice.room.android.data.local

/**
 * [KickCooldownStore] 的测试 Fake 实现（T-30042）
 *
 * 对 [InMemoryKickCooldownStore] 的简单包装，同时暴露 [savedEntries] 供测试断言使用。
 */
class FakeKickCooldownStore : KickCooldownStore {

    /** 已写入的条目，key=roomId, value=untilMs，可供测试检验 */
    val savedEntries = mutableMapOf<String, Long>()

    override fun save(roomId: String, untilMs: Long) {
        savedEntries[roomId] = untilMs
    }

    override fun get(roomId: String): Long = savedEntries[roomId] ?: 0L
}
