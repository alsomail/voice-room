package com.voice.room.android.data.local

/**
 * 被踢 cooldown 本地存储接口（T-30042）
 *
 * 记录用户被踢出某房间后的禁止进入截止时间戳（毫秒）。
 * 生产环境可用 DataStore 实现持久化；测试环境注入 [InMemoryKickCooldownStore]。
 */
interface KickCooldownStore {

    /**
     * 保存指定房间的 cooldown 截止时间。
     *
     * @param roomId  目标房间 ID
     * @param untilMs 可重新进入的时间戳（epoch 毫秒），通常为 `now + cooldown_sec * 1000`
     */
    fun save(roomId: String, untilMs: Long)

    /**
     * 获取指定房间的 cooldown 截止时间。
     *
     * @param roomId 目标房间 ID
     * @return 截止时间戳（毫秒）；若无记录则返回 0
     */
    fun get(roomId: String): Long
}

/**
 * 基于内存 Map 的 [KickCooldownStore] 实现。
 *
 * 用于：
 * - 单元测试的 Fake 注入
 * - 生产环境的简化实现（进程重启后丢失，符合 MVP 要求）
 */
class InMemoryKickCooldownStore : KickCooldownStore {

    private val map = mutableMapOf<String, Long>()

    override fun save(roomId: String, untilMs: Long) {
        map[roomId] = untilMs
    }

    override fun get(roomId: String): Long = map[roomId] ?: 0L
}
