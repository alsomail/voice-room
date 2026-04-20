package com.voice.room.android.feature.profile

/**
 * ProfileEvent — 个人中心一次性事件（由 SharedFlow 发射）
 *
 * - [NavigateToLogin]：退出登录后，导航回登录页
 * - [ShowToast]：显示 Toast 提示
 */
sealed interface ProfileEvent {
    /** 退出登录后跳转登录页 */
    data object NavigateToLogin : ProfileEvent

    /**
     * 显示 Toast 消息
     *
     * @param message 提示文本
     */
    data class ShowToast(val message: String) : ProfileEvent
}
