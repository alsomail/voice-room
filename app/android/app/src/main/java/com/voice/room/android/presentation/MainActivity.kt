package com.voice.room.android.presentation

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.voice.room.android.VoiceRoomApplication
import com.voice.room.android.core.theme.MenaTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val appContainer = (application as VoiceRoomApplication).appContainer

        setContent {
            MenaTheme {
                AppNavGraph(appContainer = appContainer)
            }
        }
    }
}
