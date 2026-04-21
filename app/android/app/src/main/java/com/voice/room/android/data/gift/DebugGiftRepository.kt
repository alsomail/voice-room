package com.voice.room.android.data.gift

import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.IGiftRepository

class DebugGiftRepository : IGiftRepository {
    override fun featuredGiftLabel(): String = "Gift module reserved"

    override suspend fun listGifts(locale: String): Result<List<GiftVO>> =
        Result.success(emptyList())
}
