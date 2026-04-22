package com.voice.room.android.core.consent

import com.voice.room.android.core.analytics.ConsentMode

/**
 * 同意模式持久化存储接口（T-30035）
 *
 * 生产实现：[DataStoreConsentStore]（基于 DataStore Preferences）
 * 测试实现：[InMemoryConsentStore]
 */
interface ConsentStore {
    /** 读取已保存的同意模式（未设置时返回 null） */
    suspend fun load(): ConsentMode?

    /** 持久化同意模式 */
    suspend fun save(mode: ConsentMode)
}

/**
 * 内存同意存储（测试用）
 */
class InMemoryConsentStore : ConsentStore {
    @Volatile
    private var stored: ConsentMode? = null

    override suspend fun load(): ConsentMode? = stored
    override suspend fun save(mode: ConsentMode) { stored = mode }
}
