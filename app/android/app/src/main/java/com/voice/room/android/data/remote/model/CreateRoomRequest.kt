package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * POST /api/v1/rooms 请求体 (T-30007 + T-30036)
 *
 * 对应 protocol.md §3.1 Request Body
 *
 * @param title        房间标题（1–30 Unicode 字符）
 * @param roomType     房间类型枚举：`normal` / `password` / `paid`
 * @param coverUrl     封面图 URL（T-30036 新增）
 * @param category     房间分类 key（T-30036 新增）：chat / emotion / music / game / matchmaking / other
 * @param announcement 公告（T-30036 新增，可选，最多 200 字符）
 * @param password     密码（`room_type=password` 时必填；其余类型忽略）
 */
data class CreateRoomRequest(
    @SerializedName("title")        val title: String,
    @SerializedName("room_type")    val roomType: String,
    @SerializedName("cover_url")    val coverUrl: String? = null,
    @SerializedName("category")     val category: String? = null,
    @SerializedName("announcement") val announcement: String? = null,
    @SerializedName("password")     val password: String? = null
)
