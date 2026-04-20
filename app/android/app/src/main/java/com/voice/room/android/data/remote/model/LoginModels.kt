package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/** POST /api/v1/auth/login 请求体 */
data class LoginRequest(
    @SerializedName("phone") val phone: String,
    @SerializedName("code") val code: String
)

/** POST /api/v1/auth/login 成功响应中的 user 对象 */
data class UserDto(
    @SerializedName("id") val id: String,
    @SerializedName("phone") val phone: String,
    @SerializedName("nickname") val nickname: String,
    @SerializedName("avatar") val avatar: String?,
    @SerializedName("coin_balance") val coinBalance: Long,
    @SerializedName("vip_level") val vipLevel: Int,
    @SerializedName("is_new") val isNew: Boolean,
    @SerializedName("created_at") val createdAt: String
)

/** POST /api/v1/auth/login 成功响应 data 字段 */
data class LoginResponseData(
    @SerializedName("token") val token: String,
    @SerializedName("expires_in") val expiresIn: Long,
    @SerializedName("user") val user: UserDto
)
