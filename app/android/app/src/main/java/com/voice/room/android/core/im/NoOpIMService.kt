package com.voice.room.android.core.im

class NoOpIMService : IIMService {
    override fun providerName(): String = "im-adapter-pending"
}
