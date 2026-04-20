package com.voice.room.android.feature.room

/**
 * 房间页整体 UI 状态（T-30009）
 *
 * 纯 data class，不含 ViewModel / 业务逻辑。
 * ViewModel 层（T-30010）负责将服务端数据映射到此状态。
 */
data class RoomUiState(
    val roomId: String = "",
    val roomName: String = "",
    val onlineCount: Int = 0,
    /** 固定 9 个麦位（index 0–8），未占用保持默认空值 */
    val micSlots: List<MicSlotUi> = List(9) { MicSlotUi(index = it) },
    val messages: List<ChatMessageUi> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
    /**
     * 是否正在发送消息（T-30016）
     *
     * `true` 时 [com.voice.room.android.feature.room.ChatInputBar] 的发送按钮禁用。
     * 由 [RoomViewModel.sendMessage] 在发送前设为 `true`，`finally` 中复位为 `false`。
     */
    val isSendingMessage: Boolean = false,
)

/**
 * 单个麦位 UI 状态
 *
 * @param index     0–8，对应 9 宫格位置
 * @param userId    null = 空麦
 * @param isMuted   是否静音（仅有人时有意义）
 */
data class MicSlotUi(
    val index: Int,
    val userId: String? = null,
    val nickname: String? = null,
    val avatarUrl: String? = null,
    val isMuted: Boolean = false,
) {
    /** true 表示该麦位有人 */
    val isOccupied: Boolean get() = userId != null
}

/**
 * 聊天消息类型（T-30014）
 */
enum class MessageType {
    /** 普通用户文字消息（左对齐，显示昵称） */
    USER_TEXT,

    /** 系统通知（居中，灰色，无昵称头像） */
    SYSTEM_NOTICE,
}

/**
 * 聊天消息 UI 模型（T-30014 扩展）
 *
 * [senderNickname] 对 [MessageType.SYSTEM_NOTICE] 可为 null。
 * [messageType] 默认为 [MessageType.USER_TEXT]，保持向后兼容。
 */
data class ChatMessageUi(
    val messageId: String,
    val senderNickname: String? = null,
    val content: String,
    val timestamp: Long,
    val messageType: MessageType = MessageType.USER_TEXT,
)
