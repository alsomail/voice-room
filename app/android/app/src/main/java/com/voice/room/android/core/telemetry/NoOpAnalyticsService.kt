package com.voice.room.android.core.telemetry

class NoOpAnalyticsService : IAnalyticsService {
    override fun trackScreen(screenName: String) = Unit

    override fun trackAction(actionName: String) = Unit
}
