package com.voice.room.android.feature.room

/**
 * 房间页一次性 UI 事件（T-30010）
 *
 * 通过 [RoomViewModel.events] Channel 发出，UI 层 LaunchedEffect 消费。
 * 与 [RoomViewState] 的区别：事件只消费一次，不持久化于状态流。
 */
sealed class RoomEvent {

    /** 离开/关闭房间，触发导航返回上一页 */
    object NavigateBack : RoomEvent()

    /**
     * 显示短暂提示（Snackbar / Toast）
     * @param message 提示内容
     */
    data class ShowToast(val message: String) : RoomEvent()

    /**
     * 通知 UI 清空聊天输入框（T-30016）
     *
     * 发送成功后由 [RoomViewModel.sendMessage] 发出，
     * [RoomScreen] 收到后将 localInputText 重置为空字符串。
     */
    object ClearInput : RoomEvent()

    /**
     * 弹出确认任命管理员对话框（T-30040 UA40-07）
     *
     * @param targetUserId 被任命目标的用户 ID
     * @param targetNickname 被任命目标的昵称（用于展示）
     */
    data class ShowConfirmAssignAdmin(
        val targetUserId: String,
        val targetNickname: String = "",
    ) : RoomEvent()

    /**
     * 弹出确认卸任管理员对话框（T-30040）
     *
     * @param targetUserId 被卸任目标的用户 ID
     * @param targetNickname 被卸任目标的昵称（用于展示）
     */
    data class ShowConfirmRevokeAdmin(
        val targetUserId: String,
        val targetNickname: String = "",
    ) : RoomEvent()

    /**
     * 弹出禁麦/禁言时长选择对话框（T-30040 UA40-09）
     *
     * @param targetUserId 被禁目标的用户 ID
     * @param muteType     禁用类型："mic" 或 "chat"
     */
    data class ShowMuteDurationDialog(
        val targetUserId: String,
        val muteType: String,
    ) : RoomEvent()
}

