package com.voice.room.android.core.network

import com.voice.room.android.domain.local.ITokenManager
import okhttp3.Interceptor
import okhttp3.Response
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.TimeUnit

class AppHttpClientFactoryTest {

    @Test
    fun `create applies timeout and retry defaults`() {
        val client = AppHttpClientFactory.create(
            NetworkClientConfig(
                connectTimeoutSeconds = 7,
                readTimeoutSeconds = 11,
                writeTimeoutSeconds = 13,
                retryOnConnectionFailure = true
            )
        )

        assertEquals(7, client.connectTimeoutMillis.toLong() / TimeUnit.SECONDS.toMillis(1))
        assertEquals(11, client.readTimeoutMillis.toLong() / TimeUnit.SECONDS.toMillis(1))
        assertEquals(13, client.writeTimeoutMillis.toLong() / TimeUnit.SECONDS.toMillis(1))
        assertTrue(client.retryOnConnectionFailure)
    }

    @Test
    fun `when authInterceptor is provided, it is registered in the client`() {
        val fakeTokenManager = object : ITokenManager {
            override suspend fun saveToken(token: String) {}
            override suspend fun getToken(): String? = null
            override suspend fun clearToken() {}
        }
        val fakeHandler = object : UnauthorizedHandler {
            override suspend fun onUnauthorized() {}
            override fun resetUnauthorized() {}
        }
        val authInterceptor = AuthInterceptor(fakeTokenManager, fakeHandler)

        val client = AppHttpClientFactory.create(
            authInterceptor = authInterceptor
        )

        assertTrue(
            "AuthInterceptor should be present in client.interceptors",
            client.interceptors.any { it is AuthInterceptor }
        )
    }

    @Test
    fun `when no authInterceptor, client has no application interceptors`() {
        val client = AppHttpClientFactory.create()

        assertTrue(
            "No interceptors should be added when authInterceptor is null",
            client.interceptors.isEmpty()
        )
    }
}
