package com.voice.room.android.data.room

import com.voice.room.android.domain.room.IRoomGateway

class DebugRoomGateway : IRoomGateway {
    override fun roomPreviewLabel(): String = "Room module reserved"
}
