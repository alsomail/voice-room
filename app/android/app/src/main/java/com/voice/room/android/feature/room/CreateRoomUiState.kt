package com.voice.room.android.feature.room

import com.voice.room.android.util.UiText

/**
 * 创建房间对话框 UI 状态 (T-30007)
 *
 * | 状态     | 含义                              |
 * |---------|----------------------------------|
 * | Idle    | 初始态：未做任何操作                  |
 * | Loading | 正在调用 API 创建房间（按钮禁用）       |
 * | Success | 创建成功，携带 [roomId]（触发导航）     |
 * | Error   | 校验失败或 API 失败，携带 [message]   |
 */
sealed interface CreateRoomUiState {
    /** 初始态 */
    object Idle : CreateRoomUiState

    /** 正在提交 — UI 按钮禁用 + 显示 CircularProgressIndicator */
    object Loading : CreateRoomUiState

    /**
     * 创建成功
     *
     * @param roomId 服务端返回的新房间 ID
     */
    data class Success(val roomId: String) : CreateRoomUiState

    /**
     * 失败（输入校验 或 API 错误）
     *
     * 缺陷 #4 修复：[message] 改为 [UiText]（@StringRes + format args），UI 层渲染。
     *
     * @param message 国际化文案占位
     */
    data class Error(val message: UiText) : CreateRoomUiState
}

