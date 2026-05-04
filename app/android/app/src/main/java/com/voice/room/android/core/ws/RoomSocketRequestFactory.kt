package com.voice.room.android.core.ws

import okhttp3.Request

data class RoomSocketRequestSpec(
    val url: String,
    val headers: Map<String, String>
) {
    internal fun toOkHttpRequest(): Request {
        val httpCompatibleUrl = when {
            url.startsWith("wss://") -> "https://${url.removePrefix("wss://")}"
            url.startsWith("ws://") -> "http://${url.removePrefix("ws://")}"
            else -> url
        }

        val requestBuilder = Request.Builder().url(httpCompatibleUrl)

        headers.forEach { (name, value) ->
            requestBuilder.addHeader(name, value)
        }

        return requestBuilder.build()
    }
}

object RoomSocketRequestFactory {
    fun create(baseWsUrl: String, session: RoomSocketSession): RoomSocketRequestSpec {
        val normalizedBaseUrl = baseWsUrl.trim().trimEnd('/')

        // Protocol §6.1: ws://host/ws?token=<JWT>
        // Token is passed as a URL query parameter; no Authorization header is used.
        return RoomSocketRequestSpec(
            url = "$normalizedBaseUrl?token=${session.accessToken}",
            headers = mapOf(
                "X-Join-Ticket" to session.joinTicket
            )
        )
    }
}
