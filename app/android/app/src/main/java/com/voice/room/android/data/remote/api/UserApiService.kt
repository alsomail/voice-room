package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.UserMeResponseData
import retrofit2.Response
import retrofit2.http.GET

/**
 * Retrofit 用户接口定义（匹配 protocol.md §2.3）
 *
 * 返回 [Response] 包装体，允许 Repository 层区分 HTTP 成功/失败并解析错误体，
 * 而不依赖 Retrofit 对 4xx 自动抛出 HttpException。
 *
 * Authorization header 由 [com.voice.room.android.core.network.AuthInterceptor] 自动注入。
 */
interface UserApiService {

    /**
     * GET /api/v1/users/me
     * 获取当前登录用户信息（需要 JWT 认证）
     */
    @GET("users/me")
    suspend fun getMe(): Response<ApiResponse<UserMeResponseData>>
}
