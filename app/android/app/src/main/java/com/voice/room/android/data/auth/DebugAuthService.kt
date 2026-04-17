package com.voice.room.android.data.auth

import com.voice.room.android.domain.auth.IAuthService

class DebugAuthService : IAuthService {
    override fun currentUserLabel(): String = "Guest bootstrap user"
}
