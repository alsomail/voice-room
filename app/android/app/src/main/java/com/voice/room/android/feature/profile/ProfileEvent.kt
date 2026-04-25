package com.voice.room.android.feature.profile

import com.voice.room.android.util.UiText

/**
 * ProfileEvent — 个人中心一次性事件（由 SharedFlow 发射）
 *
 * - [NavigateToLogin]：退出登录后，导航回登录页
 * - [ShowToast]：显示 Toast 提示（缺陷 #2 修复：text 改为 [UiText]，避免硬编码语言）
 */
sealed interface ProfileEvent {
    /** 退出登录后跳转登录页 */
    data object NavigateToLogin : ProfileEvent

    /**
     * 显示 Toast 消息
     *
     * @param message 提示文本 [UiText]（@StringRes + 可选 format 参数），
     *                由 UI 层在 Composable / Activity 中通过 `asString(context)` 解析为目标 Locale
     */
    data class ShowToast(val message: UiText) : ProfileEvent
}
