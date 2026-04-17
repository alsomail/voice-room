package com.voice.room.android.core.telemetry

interface ICrashReporter {
    fun recordNonFatal(message: String)
}
