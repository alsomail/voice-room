package com.voice.room.android.data.gift

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.GiftApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.GiftDto
import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.IGiftRepository

/**
 * [IGiftRepository] 的 Retrofit 实现，带 60 秒内存缓存 (T-30028)
 *
 * ### 缓存策略
 * - 打开面板时调用 [listGifts]
 * - 若 `System.currentTimeMillis() - cacheTimestamp < 60_000`，直接返回 [cachedGifts]
 * - 否则发起 `GET /api/v1/gifts/list`，成功后更新缓存
 *
 * ### 错误处理
 * - 2xx + code==0 → 返回 [GiftVO] 列表
 * - 2xx + code≠0  → 抛出 [ApiException]
 * - 4xx/5xx       → 解析 error body，抛出 [ApiException]
 */
class RetrofitGiftRepository(
    private val apiService: GiftApiService,
) : IGiftRepository {

    private val gson = Gson()

    /** 内存缓存：上次成功响应的礼物列表 */
    @Volatile private var cachedGifts: List<GiftVO>? = null

    /** 上次缓存写入时间戳（毫秒） */
    @Volatile private var cacheTimestamp: Long = 0L

    /** 缓存有效期：60 秒 */
    internal val cacheDurationMs: Long = 60_000L

    override fun featuredGiftLabel(): String = "Gift (Retrofit)"

    // ─── listGifts ───────────────────────────────────────────────────────────

    override suspend fun listGifts(locale: String): Result<List<GiftVO>> = runCatching {
        val now = System.currentTimeMillis()
        val cached = cachedGifts
        if (cached != null && (now - cacheTimestamp) < cacheDurationMs) {
            return@runCatching cached
        }

        val response = apiService.listGifts(acceptLanguage = locale)
        val dtos = parseBody(response)
        val gifts = dtos.map { it.toDomain() }

        cachedGifts = gifts
        cacheTimestamp = System.currentTimeMillis()
        gifts
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    private fun <T> parseBody(response: retrofit2.Response<ApiResponse<T>>): T {
        if (response.isSuccessful) {
            val apiBody = response.body()
                ?: throw ApiException(-1, "Empty response body")
            if (apiBody.code == 0 && apiBody.data != null) {
                return apiBody.data
            }
            throw ApiException(apiBody.code, apiBody.message)
        }

        val errorJson = response.errorBody()?.string()
        if (!errorJson.isNullOrBlank()) {
            runCatching {
                val type = object : TypeToken<ApiResponse<Nothing>>() {}.type
                val errorBody: ApiResponse<Nothing> = gson.fromJson(errorJson, type)
                throw ApiException(errorBody.code, errorBody.message)
            }.onSuccess { /* unreachable */ }
                .onFailure { if (it is ApiException) throw it }
        }
        throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
    }

    // ─── DTO → Domain ─────────────────────────────────────────────────────────

    private fun GiftDto.toDomain(): GiftVO = GiftVO(
        id = id,
        code = code,
        name = name,
        iconUrl = iconUrl,
        price = price,
        sortOrder = sortOrder,
        tier = tier,
    )
}
