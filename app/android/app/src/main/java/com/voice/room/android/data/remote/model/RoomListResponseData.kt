package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 房间列表分页 DTO — 对应 protocol.md §3.2 `data` 字段
 */
data class RoomListResponseData(
    @SerializedName("total") val total: Int,
    @SerializedName("page")  val page: Int,
    @SerializedName("size")  val size: Int,
    @SerializedName("items") val items: List<RoomItemDto>
)
