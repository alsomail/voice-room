package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.GiftDto
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.Header

/**
 * 礼物 Retrofit API 接口 (T-30028)
 *
 * Base URL：`/api/v1/`
 * Authorization header 由 AuthInterceptor 自动注入
 */
interface GiftApiService {

    /**
     * 获取礼物列表（已按 sort_order 排序）
     *
     * GET /api/v1/gifts/list
     * Header: Accept-Language: {locale}
     *
     * @param acceptLanguage IETF 语言标签（如 "en"、"ar"）
     */
    @GET("gifts/list")
    suspend fun listGifts(
        @Header("Accept-Language") acceptLanguage: String = "en",
    ): Response<ApiResponse<List<GiftDto>>>
}
