package com.voice.room.android.feature.room

/**
 * 被踢出房间后的状态（T-30042）
 *
 * 由 [RoomViewModel.kickedState] 暴露，UserKickedDialog 消费。
 *
 * @param reason      踢出原因（来自服务端，如 "spam"/"abuse"/"harassment"）
 * @param cooldownSec 重新进入的冷却时间（秒），默认 600（10 分钟）
 */
data class KickedState(
    val reason: String,
    val cooldownSec: Int = 600,
)
