package com.voice.room.android.domain.gift

/**
 * 麦位用户值对象（接收者槽使用，T-30028 / T-30029）
 *
 * 从 [com.voice.room.android.feature.room.MicSlotUi] 中提取已占用麦位用户信息。
 * 仅包含 on-mic 的用户（slot_index != null），空麦位不传入。
 *
 * @param userId    用户 ID
 * @param nickname  显示昵称
 * @param avatarUrl 头像 URL（可为 null）
 * @param micIndex  麦位序号（slot_index，0 = 主麦）；默认 0，供 T-30029 排序使用
 */
data class MicUserVO(
    val userId: String,
    val nickname: String,
    val avatarUrl: String?,
    val micIndex: Int = 0,
)
