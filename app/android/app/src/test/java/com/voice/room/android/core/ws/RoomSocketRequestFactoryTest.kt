package com.voice.room.android.core.ws

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class RoomSocketRequestFactoryTest {
    @Test
    fun `create builds URL with token query param and no room path`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "ws://192.168.1.19:3000/ws",
            session = RoomSocketSession(
                accessToken = "token-123",
                joinTicket = "ticket-456"
            )
        )

        assertEquals("ws://192.168.1.19:3000/ws?token=token-123", request.url)
        assertFalse(request.headers.containsKey("Authorization"))
        assertEquals("ticket-456", request.headers["X-Join-Ticket"])
    }

    @Test
    fun `toOkHttpRequest converts wss scheme and preserves token query param`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "wss://voice-room.example.com/ws",
            session = RoomSocketSession(
                accessToken = "secure-token",
                joinTicket = "secure-ticket",
                roomPath = "sync"   // roomPath is intentionally ignored per §6.1
            )
        )

        val okHttpRequest = request.toOkHttpRequest()

        assertEquals("https://voice-room.example.com/ws?token=secure-token", okHttpRequest.url.toString())
        assertFalse(okHttpRequest.headers.names().contains("Authorization"))
        assertEquals("secure-ticket", okHttpRequest.header("X-Join-Ticket"))
    }

    // ── Round 9: BUG-CHAT-WS 协议契约修复 ──────────────────────────────────────

    @Test
    fun `RF-R9-01 create URL places token as query param and omits room path suffix`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "ws://192.168.1.19:3000/ws",
            session = RoomSocketSession(
                accessToken = "token-abc",
                joinTicket = "ticket-xyz"
            )
        )

        assertEquals(
            "URL must be baseWsUrl?token=<JWT> with no /room suffix",
            "ws://192.168.1.19:3000/ws?token=token-abc",
            request.url
        )
        assertFalse("URL must not contain /room", request.url.contains("/room"))
    }

    @Test
    fun `RF-R9-02 create headers do not contain Authorization field`() {
        val request = RoomSocketRequestFactory.create(
            baseWsUrl = "ws://192.168.1.19:3000/ws",
            session = RoomSocketSession(
                accessToken = "token-abc",
                joinTicket = "ticket-xyz"
            )
        )

        assertFalse(
            "Authorization header must not be present — token is in URL query param",
            request.headers.containsKey("Authorization")
        )
        assertEquals("ticket-xyz", request.headers["X-Join-Ticket"])
    }
}
