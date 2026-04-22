package com.voice.room.android.data.model

/**
 * 房间成员数据模型（T-30039）
 *
 * 用于观众席 BottomSheet 展示麦上成员和观众列表。
 *
 * @param id        用户 ID（唯一键）
 * @param nickname  用户昵称
 * @param avatarUrl 头像 URL（可为 null）
 * @param role      角色："owner" / "admin" / "member"
 * @param slot      麦位下标（0–8），null 表示未上麦（观众）
 * @param joinedAt  加入时间戳（毫秒，Unix epoch）
 * @param micMuted  是否被禁麦
 * @param chatMuted 是否被禁言
 */
data class RoomMember(
    val id: String,
    val nickname: String,
    val avatarUrl: String? = null,
    val role: String = "member",
    val slot: Int? = null,
    val joinedAt: Long = 0L,
    val micMuted: Boolean = false,
    val chatMuted: Boolean = false,
)
