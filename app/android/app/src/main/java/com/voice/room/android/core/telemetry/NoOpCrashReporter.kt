package com.voice.room.android.core.telemetry

class NoOpCrashReporter : ICrashReporter {
    override fun recordNonFatal(message: String) = Unit
}
