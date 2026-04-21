package com.voice.room.android.domain.gift

/**
 * 麦位用户值对象（接收者槽使用，T-30028）
 *
 * 从 [com.voice.room.android.feature.room.MicSlotUi] 中提取已占用麦位用户信息。
 *
 * @param userId    用户 ID
 * @param nickname  显示昵称
 * @param avatarUrl 头像 URL（可为 null）
 */
data class MicUserVO(
    val userId: String,
    val nickname: String,
    val avatarUrl: String?,
)
