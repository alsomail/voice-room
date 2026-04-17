package com.voice.room.android.core.network

data class NetworkClientConfig(
    val connectTimeoutSeconds: Long = 10,
    val readTimeoutSeconds: Long = 20,
    val writeTimeoutSeconds: Long = 20,
    val retryOnConnectionFailure: Boolean = true
)
