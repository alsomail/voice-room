package com.voice.room.android.feature.splash

/**
 * Splash 页导航事件（单次消费）
 *
 * 通过 [SplashViewModel.navEvent]（SharedFlow）向 UI 层发送，
 * UI 层在 LaunchedEffect 中收集并驱动 NavController 导航。
 */
sealed class SplashNavEvent {
    /** JWT 有效 → 跳转主页 */
    object NavigateToMain : SplashNavEvent()

    /** JWT 无效/缺失/读取异常 → 跳转登录页 */
    object NavigateToLogin : SplashNavEvent()
}
