package com.voice.room.android.data.auth

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.remote.api.AuthApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.LoginRequest
import com.voice.room.android.data.remote.model.SendCodeRequest
import com.voice.room.android.domain.auth.IAuthRepository
import com.voice.room.android.domain.auth.LoginResult
import com.voice.room.android.domain.auth.SendCodeResult

/**
 * [IAuthRepository] 的 Retrofit 真实实现
 *
 * - 通过 [AuthApiService] 发起 HTTP 请求
 * - HTTP 2xx → 检查 `code==0`，映射为领域对象
 * - HTTP 4xx / 5xx → 解析 error body 为 [ApiException]
 * - 网络异常 → 原样封装为 [Result.failure]
 */
class RetrofitAuthRepository(
    private val apiService: AuthApiService
) : IAuthRepository {

    private val gson = Gson()

    override suspend fun sendCode(phone: String): Result<SendCodeResult> = runCatching {
        val httpResponse = apiService.sendCode(SendCodeRequest(phone))
        val body = parseBody<com.voice.room.android.data.remote.model.SendCodeResponseData>(httpResponse)
        SendCodeResult(cooldownSeconds = body.cooldown)
    }

    override suspend fun login(phone: String, code: String): Result<LoginResult> = runCatching {
        val httpResponse = apiService.login(LoginRequest(phone, code))
        val body = parseBody<com.voice.room.android.data.remote.model.LoginResponseData>(httpResponse)
        LoginResult(
            token = body.token,
            userId = body.user.id,
            isNew = body.user.isNew
        )
    }

    // ─────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────

    /**
     * 解析 HTTP 响应：
     * - 2xx + code==0 → 返回 data
     * - 2xx + code≠0  → 抛出 [ApiException]（不应发生，protocol 规定 2xx 只有 code=0）
     * - 4xx/5xx       → 解析 error body，抛出 [ApiException]
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

        // HTTP 4xx/5xx — try to parse error body
        val errorJson = response.errorBody()?.string()
        if (!errorJson.isNullOrBlank()) {
            runCatching {
                val type = object : TypeToken<ApiResponse<Nothing>>() {}.type
                val errorBody: ApiResponse<Nothing> = gson.fromJson(errorJson, type)
                throw ApiException(errorBody.code, errorBody.message)
            }.onSuccess { /* unreachable – throw above */ }
                .onFailure { if (it is ApiException) throw it }
        }
        throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
    }
}
