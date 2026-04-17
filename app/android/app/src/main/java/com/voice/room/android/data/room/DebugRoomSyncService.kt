package com.voice.room.android.data.room

import com.voice.room.android.domain.room.IRoomSyncService

class DebugRoomSyncService : IRoomSyncService {
    override fun syncPolicyLabel(): String = "Heartbeat and reconnect planned"
}
