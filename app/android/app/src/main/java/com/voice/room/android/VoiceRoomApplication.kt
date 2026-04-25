package com.voice.room.android

import android.app.Application
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.ProcessLifecycleOwner
import com.voice.room.android.common.AppContainer

/**
 * Application 入口（T-30035 / R1 批 2 缺陷 2）
 *
 * 装配 [AppContainer]（含完整 Analytics 主链路），并注册 [ProcessLifecycleOwner] 观察者：
 * - **ON_START**（前台）→ [com.voice.room.android.core.analytics.session.SessionManager.onForeground]
 * - **ON_STOP** （后台）→ [com.voice.room.android.core.analytics.throttle.Throttler.onStop]
 *   触发 flush，并通知 SessionManager 进入后台计时（≥30s 后回前台生成新 session）
 */
class VoiceRoomApplication : Application() {
    lateinit var appContainer: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        appContainer = AppContainer.fromBuildConfig(applicationContext)

        // 进程级前后台生命周期 → SessionManager + Throttler
        ProcessLifecycleOwner.get().lifecycle.addObserver(
            LifecycleEventObserver { _, event ->
                when (event) {
                    Lifecycle.Event.ON_START -> appContainer.sessionManager.onForeground()
                    Lifecycle.Event.ON_STOP -> {
                        appContainer.sessionManager.onBackground()
                        appContainer.throttler.onStop()
                    }
                    else -> Unit
                }
            }
        )
    }
}
