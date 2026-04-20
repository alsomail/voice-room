package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 单房间 DTO — 对应 protocol.md §3.2 items[] 单项
 */
data class RoomItemDto(
    @SerializedName("room_id")        val roomId: String,
    @SerializedName("title")          val title: String,
    @SerializedName("room_type")      val roomType: String,
    @SerializedName("member_count")   val memberCount: Int,
    @SerializedName("max_members")    val maxMembers: Int,
    @SerializedName("owner_id")       val ownerId: String,
    @SerializedName("owner_nickname") val ownerNickname: String,
    @SerializedName("owner_avatar")   val ownerAvatar: String?,
    @SerializedName("created_at")     val createdAt: String
)
