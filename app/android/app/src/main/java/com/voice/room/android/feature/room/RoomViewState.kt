package com.voice.room.android.feature.room

/**
 * 房间页 ViewModel 状态（T-30010）
 *
 * 通过 [RoomViewModel.uiState] 暴露，UI 层 collectAsState() 驱动渲染。
 */
sealed class RoomViewState {

    /** 正在加载（进入房间、获取快照期间） */
    object Loading : RoomViewState()

    /**
     * 加载成功，包含完整 UI 状态
     * @param uiState 当前房间 UI 快照
     */
    data class Success(val uiState: RoomUiState) : RoomViewState()

    /**
     * 加载失败
     * @param message 面向开发者的错误描述（非空）
     */
    data class Error(val message: String) : RoomViewState()
}
