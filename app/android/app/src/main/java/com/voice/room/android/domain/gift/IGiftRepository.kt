package com.voice.room.android.domain.gift

/**
 * 礼物仓库接口（防腐层，T-30028）
 *
 * 生产实现：[com.voice.room.android.data.gift.RetrofitGiftRepository]（带 60s 内存缓存）
 * 调试实现：[com.voice.room.android.data.gift.DebugGiftRepository]
 * 测试 Fake：在各测试文件内定义 FakeGiftRepository
 */
interface IGiftRepository {
    /** 礼物模块预览标签（AppContainer 检查用） */
    fun featuredGiftLabel(): String

    /**
     * 获取礼物列表。
     *
     * - 若内存缓存 <60s 直接返回缓存结果
     * - 否则 `GET /api/v1/gifts/list`，带 `Accept-Language: {locale}` Header
     * - 后端按 `sort_order` 排序
     *
     * @param locale IETF 语言标签（如 "en"、"ar"），对应 HTTP Accept-Language
     * @return [Result.success] 包含有序的 [GiftVO] 列表；[Result.failure] 包含异常
     */
    suspend fun listGifts(locale: String = "en"): Result<List<GiftVO>>
}
