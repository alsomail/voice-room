package com.voice.room.android.presentation

import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import com.voice.room.android.R
import com.voice.room.android.VoiceRoomApplication

class MainActivity : AppCompatActivity() {
    private val viewModel: MainViewModel by viewModels {
        val container = (application as VoiceRoomApplication).appContainer
        MainViewModel.Factory(container)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        bindButtons()
        render(viewModel.uiState)
    }

    private fun bindButtons() {
        findViewById<Button>(R.id.authButton).setOnClickListener {
            render(viewModel.onDestinationSelected(MainDestination.AUTH))
        }
        findViewById<Button>(R.id.roomButton).setOnClickListener {
            render(viewModel.onDestinationSelected(MainDestination.ROOM))
        }
        findViewById<Button>(R.id.profileButton).setOnClickListener {
            render(viewModel.onDestinationSelected(MainDestination.PROFILE))
        }
    }

    private fun render(state: MainUiState) {
        findViewById<TextView>(R.id.screenTitle).text = state.title
        findViewById<TextView>(R.id.screenDescription).text = state.description
        findViewById<TextView>(R.id.apiValue).text = state.apiBaseUrl
        findViewById<TextView>(R.id.wsValue).text = state.wsUrl
        findViewById<TextView>(R.id.statusValue).text = state.statusLines.joinToString(separator = "\n")
        findViewById<TextView>(R.id.warningValue).text = state.warnings.joinToString(separator = "\n")
            .ifBlank { getString(R.string.no_warning) }
    }
}
