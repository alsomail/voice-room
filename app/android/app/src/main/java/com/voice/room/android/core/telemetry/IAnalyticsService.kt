package com.voice.room.android.core.telemetry

interface IAnalyticsService {
    fun trackScreen(screenName: String)
    fun trackAction(actionName: String)
}
