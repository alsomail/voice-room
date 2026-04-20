package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * POST /api/v1/rooms 响应体 data 字段 (T-30007)
 *
 * 对应 protocol.md §3.1 Success Response data 节点。
 * ViewModel 层只需要 [roomId]，其余字段按需解析。
 *
 * @param roomId    新建房间 ID（UUID）
 * @param title     房间标题
 * @param roomType  房间类型
 * @param createdAt 创建时间（ISO-8601）
 */
data class CreateRoomResponseData(
    @SerializedName("room_id")    val roomId: String,
    @SerializedName("title")      val title: String,
    @SerializedName("room_type")  val roomType: String,
    @SerializedName("created_at") val createdAt: String
)
