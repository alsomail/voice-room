package com.voice.room.android.domain.room

/**
 * 房间领域模型（对应 protocol.md §3.2 items[] 单项）
 *
 * @param roomId         房间唯一 ID
 * @param title          房间标题
 * @param roomType       房间类型："normal" / "password" / "paid"
 * @param memberCount    当前在线人数
 * @param maxMembers     最大人数
 * @param ownerNickname  房主昵称
 * @param ownerAvatar    房主头像 URL（null 时 Coil 显示占位图）
 * @param createdAt      创建时间（ISO 8601）
 */
data class RoomItem(
    val roomId: String,
    val title: String,
    val roomType: String,
    val memberCount: Int,
    val maxMembers: Int,
    val ownerNickname: String,
    val ownerAvatar: String?,
    val createdAt: String
)
