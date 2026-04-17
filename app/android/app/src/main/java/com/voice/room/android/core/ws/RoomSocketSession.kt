package com.voice.room.android.core.ws

data class RoomSocketSession(
    val accessToken: String,
    val joinTicket: String,
    val roomPath: String = "/room"
)
