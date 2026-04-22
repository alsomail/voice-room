package com.voice.room.android.feature.room

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
     * 显示短暂提示（Toast / Snackbar）
     *
     * @param message 提示内容
     */
    data class ShowToast(val message: String) : HallEvent()
}
