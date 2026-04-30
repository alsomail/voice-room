package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * GET /api/v1/rooms/{id} 响应体 data 字段（BUG-ROOM-NAV 修复）
 *
 * 对应服务端 `RoomDetailResponse`（modules/room/dto.rs）。
 * 用于 [com.voice.room.android.data.room.RetrofitRoomSnapshotRepository] 映射为 [com.voice.room.android.data.room.RoomSnapshot]。
 *
 * @param roomId      房间 UUID
 * @param title       房间标题
 * @param roomType    房间类型（normal / password / paid）
 * @param memberCount 当前在线人数
 * @param maxMembers  最大成员数
 * @param micSlots    麦位数组（MVP 服务端固定返回空数组，Android 端补 9 个空位）
 * @param createdAt   创建时间（ISO-8601）
 */
data class RoomDetailResponseData(
    @SerializedName("room_id")      val roomId: String,
    @SerializedName("title")        val title: String,
    @SerializedName("room_type")    val roomType: String,
    @SerializedName("member_count") val memberCount: Int,
    @SerializedName("max_members")  val maxMembers: Int,
    @SerializedName("mic_slots")    val micSlots: List<Any> = emptyList(),
    @SerializedName("created_at")   val createdAt: String,
)
