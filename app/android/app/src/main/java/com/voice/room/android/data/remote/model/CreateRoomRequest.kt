package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * POST /api/v1/rooms 请求体 (T-30007)
 *
 * 对应 protocol.md §3.1 Request Body
 *
 * @param title    房间标题（1–30 Unicode 字符）
 * @param roomType 房间类型枚举：`normal` / `password` / `paid`
 * @param password 密码（`room_type=password` 时必填；其余类型忽略）
 */
data class CreateRoomRequest(
    @SerializedName("title")     val title: String,
    @SerializedName("room_type") val roomType: String,
    @SerializedName("password")  val password: String? = null
)
