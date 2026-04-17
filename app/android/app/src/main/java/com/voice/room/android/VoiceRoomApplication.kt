package com.voice.room.android

import android.app.Application
import com.voice.room.android.common.AppContainer

class VoiceRoomApplication : Application() {
    lateinit var appContainer: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        appContainer = AppContainer.fromBuildConfig()
    }
}
