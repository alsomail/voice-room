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
}
