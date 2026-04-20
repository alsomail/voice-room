package com.voice.room.android.core.network

import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.runBlocking
import okhttp3.Interceptor
import okhttp3.Response

/**
 * OkHttp 应用层拦截器：JWT 自动注入 + 401 统一处理
 *
 * **职责：**
 * 1. 每次请求前从 [ITokenManager] 同步读取 Token：
 *    - Token 非空非空白 → 添加 `Authorization: Bearer {token}` Header
 *    - Token 为 null 或空白 → 直接放行（匿名请求，如 /auth/send-code、/auth/login）
 * 2. 收到 HTTP 401 响应 → 调用 [UnauthorizedHandler.onUnauthorized]（清除 Token + 发射导航事件）
 * 3. **不重试**：proceed() 只调用一次，避免无限循环。
 *
 * **线程说明：**
 * OkHttp 在后台线程调用 [intercept]，使用 [runBlocking] 在该线程上执行挂起函数是安全的，
 * 不会阻塞 Android 主线程。
 *
 * **注册方式（AppHttpClientFactory）：**
 * ```kotlin
 * OkHttpClient.Builder().addInterceptor(AuthInterceptor(tokenManager, unauthorizedHandler))
 * ```
 */
class AuthInterceptor(
    private val tokenManager: ITokenManager,
    private val unauthorizedHandler: UnauthorizedHandler
) : Interceptor {

    override fun intercept(chain: Interceptor.Chain): Response {
        // ① 同步读取 Token（runBlocking 在 OkHttp 后台线程上是安全的）
        val token: String? = runBlocking { tokenManager.getToken() }

        // ② 仅当 token 非空非空白时附加 Authorization Header
        val request = if (!token.isNullOrBlank()) {
            chain.request().newBuilder()
                .header("Authorization", "Bearer $token")
                .build()
        } else {
            chain.request()
        }

        // ③ 发出请求（只调用一次，不重试）
        val response = chain.proceed(request)

        // ④ 401 → 清除 Token + 触发跳转登录页事件
        if (response.code == 401) {
            runBlocking { unauthorizedHandler.onUnauthorized() }
        }

        return response
    }
}
