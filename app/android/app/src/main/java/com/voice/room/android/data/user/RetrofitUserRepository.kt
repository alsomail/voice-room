package com.voice.room.android.data.user

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.UserApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.UserMeResponseData
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.user.UserProfile

/**
 * [IUserRepository] 的 Retrofit 真实实现
 *
 * - 通过 [UserApiService] 调用 GET /api/v1/users/me
 * - HTTP 2xx + code==0 → 映射为 [UserProfile] 领域模型
 * - HTTP 4xx/5xx → 解析 error body 为 [ApiException]
 * - 网络异常 → 原样封装为 [Result.failure]（保留 IOException）
 *
 * Authorization header 由 AuthInterceptor 自动注入，本类无需处理 token。
 */
class RetrofitUserRepository(
    private val apiService: UserApiService
) : IUserRepository {

    private val gson = Gson()

    override suspend fun getMe(): Result<UserProfile> = runCatching {
        val httpResponse = apiService.getMe()
        val data = parseBody(httpResponse)
        data.toDomain()
    }

    // ─────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────

    /**
     * 解析 HTTP 响应（与 RetrofitAuthRepository 保持一致的错误处理策略）：
     * - 2xx + code==0 + data≠null → 返回 data
     * - 2xx + code≠0              → 抛出 [ApiException]
     * - 2xx + body 为 null        → 抛出 [ApiException]
     * - 4xx/5xx                   → 解析 error body，抛出 [ApiException]；解析失败时使用 HTTP 状态码
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

        // HTTP 4xx/5xx — 尝试解析 error body
        val errorJson = response.errorBody()?.string()
        if (!errorJson.isNullOrBlank()) {
            runCatching {
                val type = object : TypeToken<ApiResponse<Nothing>>() {}.type
                val errorBody: ApiResponse<Nothing> = gson.fromJson(errorJson, type)
                throw ApiException(errorBody.code, errorBody.message)
            }.onSuccess { /* unreachable — throw above */ }
                .onFailure { if (it is ApiException) throw it }
        }
        throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
    }

    /**
     * 将 DTO 映射为领域模型（保证 Domain 层与远程数据结构解耦）
     */
    private fun UserMeResponseData.toDomain(): UserProfile = UserProfile(
        id = id,
        phone = phone,
        nickname = nickname,
        avatar = avatar,
        coinBalance = coinBalance,
        vipLevel = vipLevel,
        createdAt = createdAt
    )
}
