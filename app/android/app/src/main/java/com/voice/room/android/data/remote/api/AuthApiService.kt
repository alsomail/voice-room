package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.LoginRequest
import com.voice.room.android.data.remote.model.LoginResponseData
import com.voice.room.android.data.remote.model.SendCodeRequest
import com.voice.room.android.data.remote.model.SendCodeResponseData
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.POST

/**
 * Retrofit 认证接口定义（匹配 protocol.md §二）
 *
 * 返回 [Response] 包装体，允许 Repository 层区分 HTTP 成功/失败并解析错误体，
 * 而不依赖 Retrofit 对 4xx 自动抛出 HttpException。
 */
interface AuthApiService {

    /**
     * POST /api/v1/auth/verification-codes
     * 发送短信验证码（无需鉴权）
     */
    @POST("auth/verification-codes")
    suspend fun sendCode(
        @Body request: SendCodeRequest
    ): Response<ApiResponse<SendCodeResponseData>>

    /**
     * POST /api/v1/auth/login
     * 手机号 + 验证码一步登录（首次注册自动创建用户）
     */
    @POST("auth/login")
    suspend fun login(
        @Body request: LoginRequest
    ): Response<ApiResponse<LoginResponseData>>
}
