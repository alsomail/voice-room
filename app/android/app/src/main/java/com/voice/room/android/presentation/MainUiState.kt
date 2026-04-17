package com.voice.room.android.presentation

data class MainUiState(
    val title: String,
    val description: String,
    val apiBaseUrl: String,
    val wsUrl: String,
    val statusLines: List<String>,
    val warnings: List<String>
)
