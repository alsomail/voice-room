package com.voice.room.android.data.gift

import com.voice.room.android.domain.gift.IGiftRepository

class DebugGiftRepository : IGiftRepository {
    override fun featuredGiftLabel(): String = "Gift module reserved"
}
