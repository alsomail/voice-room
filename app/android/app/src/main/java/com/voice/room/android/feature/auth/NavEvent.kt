package com.voice.room.android.feature.auth

/**
 * 登录页导航事件（单次消费）
 *
 * 通过 [LoginViewModel.navEvent]（SharedFlow）向 UI 层发送，
 * UI 层在 LaunchedEffect 中收集并执行导航。
 */
sealed class NavEvent {
    /** 登录成功 → 跳转大厅 */
    object NavigateToHall : NavEvent()
}
