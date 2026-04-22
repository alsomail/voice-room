package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.RankingDto
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.Query

/**
 * 榜单 Retrofit API 接口 (T-30033)
 *
 * Base URL：`/api/v1/`
 * Authorization header 由 AuthInterceptor 自动注入
 */
interface RankingApiService {

    /**
     * 查询榜单
     *
     * GET /api/v1/ranking?type={type}&period={period}&limit={limit}
     *
     * @param type   榜单类型：charm/wealth
     * @param period 榜单周期：day/week
     * @param limit  返回数量，1-100，默认 50
     */
    @GET("ranking")
    suspend fun getRanking(
        @Query("type")   type: String,
        @Query("period") period: String,
        @Query("limit")  limit: Int = 50,
    ): Response<ApiResponse<RankingDto>>
}
