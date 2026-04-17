package com.voice.room.android.core.network

import okhttp3.OkHttpClient
import java.util.concurrent.TimeUnit

internal object AppHttpClientFactory {
    internal fun create(config: NetworkClientConfig = NetworkClientConfig()): OkHttpClient {
        return OkHttpClient.Builder()
            .connectTimeout(config.connectTimeoutSeconds, TimeUnit.SECONDS)
            .readTimeout(config.readTimeoutSeconds, TimeUnit.SECONDS)
            .writeTimeout(config.writeTimeoutSeconds, TimeUnit.SECONDS)
            .retryOnConnectionFailure(config.retryOnConnectionFailure)
            .build()
    }
}
