package com.voice.room.android.core.media

class NoOpMediaService : IMediaService {
    override fun providerName(): String = "media-adapter-pending"
}
