package com.voice.room.android.feature.room

import com.voice.room.android.util.UiText

/**
 * 大厅页一次性 UI 事件（T-30038）
 *
 * 通过 [HallViewModel.hallEvents] Channel 发出，UI 层 LaunchedEffect 消费。
 */
sealed class HallEvent {

    /**
     * 导航到房间页
     *
     * @param roomId     目标房间 ID
     * @param accessToken 密码房的访问令牌（普通房为 null）
     */
    data class NavigateToRoom(
        val roomId: String,
        val accessToken: String?
    ) : HallEvent()

    /**
     * 显示短暂提示（Toast / Snackbar）。
     *
     * 缺陷 #4 修复：消息体改为 [UiText]，禁止持有任何特定语言的字面量；
     * UI 层在 Composable 中通过 `text.asString()` 解析为目标 locale 的字符串。
     *
     * @param text 国际化文案（@StringRes + 可选 format 参数）
     */
    data class ShowToast(val text: UiText) : HallEvent()
}

