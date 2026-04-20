package com.voice.room.android.core.network

import okhttp3.OkHttpClient
import java.util.concurrent.TimeUnit

/**
 * OkHttpClient 工厂
 *
 * - 通过 [config] 配置超时与重连策略（见 [NetworkClientConfig]）
 * - 通过 [authInterceptor] 注入 [AuthInterceptor]（可选，生产环境传入，测试/匿名场景可省略）
 *
 * 用法示例：
 * ```kotlin
 * val client = AppHttpClientFactory.create(
 *     authInterceptor = AuthInterceptor(tokenManager, unauthorizedHandler)
 * )
 * ```
 */
internal object AppHttpClientFactory {

    internal fun create(
        config: NetworkClientConfig = NetworkClientConfig(),
        authInterceptor: AuthInterceptor? = null
    ): OkHttpClient {
        return OkHttpClient.Builder()
            .connectTimeout(config.connectTimeoutSeconds, TimeUnit.SECONDS)
            .readTimeout(config.readTimeoutSeconds, TimeUnit.SECONDS)
            .writeTimeout(config.writeTimeoutSeconds, TimeUnit.SECONDS)
            .retryOnConnectionFailure(config.retryOnConnectionFailure)
            .apply {
                authInterceptor?.let { addInterceptor(it) }
            }
            .build()
    }
}
