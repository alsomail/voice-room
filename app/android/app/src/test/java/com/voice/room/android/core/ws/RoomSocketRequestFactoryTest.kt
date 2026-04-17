package com.voice.room.android.core.ws

import org.junit.Assert.assertEquals
import org.junit.Test

class RoomSocketRequestFactoryTest {
    @Test
    fun `create appends room path and injects auth headers`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "ws://192.168.1.8:3000/ws",
            session = RoomSocketSession(
                accessToken = "token-123",
                joinTicket = "ticket-456"
            )
        )

        assertEquals("ws://192.168.1.8:3000/ws/room", request.url)
        assertEquals("Bearer token-123", request.headers["Authorization"])
        assertEquals("ticket-456", request.headers["X-Join-Ticket"])
    }

    @Test
    fun `toOkHttpRequest converts ws scheme for OkHttp compatibility`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "wss://voice-room.example.com/ws",
            session = RoomSocketSession(
                accessToken = "secure-token",
                joinTicket = "secure-ticket",
                roomPath = "sync"
            )
        )

        val okHttpRequest = request.toOkHttpRequest()

        assertEquals("https://voice-room.example.com/ws/sync", okHttpRequest.url.toString())
        assertEquals("Bearer secure-token", okHttpRequest.header("Authorization"))
        assertEquals("secure-ticket", okHttpRequest.header("X-Join-Ticket"))
    }
}
