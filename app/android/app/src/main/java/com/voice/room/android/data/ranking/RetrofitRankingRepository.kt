package com.voice.room.android.data.ranking

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.RankingApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.RankingDto
import com.voice.room.android.domain.ranking.IRankingRepository
import com.voice.room.android.domain.ranking.MyRank
import com.voice.room.android.domain.ranking.RankEntry
import com.voice.room.android.domain.ranking.RankingPage

/**
 * [IRankingRepository] 的 Retrofit 实现 (T-30033)
 *
 * - [getRanking] → GET /api/v1/ranking?type={type}&period={period}
 * - Authorization header 由 AuthInterceptor 自动注入
 * - HTTP 4xx/5xx 统一解析为 [ApiException]
 */
class RetrofitRankingRepository(
    private val apiService: RankingApiService,
) : IRankingRepository {

    private val gson = Gson()

    // ─── getRanking ──────────────────────────────────────────────────────────

    override suspend fun getRanking(type: String, period: String): Result<RankingPage> =
        runCatching {
            val response = apiService.getRanking(type = type, period = period)
            val dto = parseBody(response)
            dto.toDomain()
        }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /**
     * 统一 HTTP 响应解析（与项目现有 Repository 保持相同错误处理策略）：
     * - 2xx + code==0 + data≠null → 返回 data
     * - 2xx + code≠0              → 抛出 [ApiException]
     * - 4xx/5xx                   → 解析 error body，抛出 [ApiException]
     */
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

    private fun RankingDto.toDomain(): RankingPage = RankingPage(
        type = type,
        period = period,
        items = items.map { it.toDomain() },
        me = me?.let { MyRank(rank = it.rank, score = it.score) },
    )

    private fun com.voice.room.android.data.remote.model.RankEntryDto.toDomain(): RankEntry =
        RankEntry(
            rank = rank,
            userId = userId,
            nickname = nickname,
            avatar = avatar,
            score = score,
            medal = medal,
        )
}
