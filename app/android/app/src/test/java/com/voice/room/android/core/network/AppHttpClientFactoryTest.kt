package com.voice.room.android.core.network

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
}
