package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * POST /api/v1/rooms/:id/verify-password 请求体（T-30038）
 *
 * @param password 用户输入的 6 位房间密码
 */
data class VerifyPasswordRequest(
    @SerializedName("password") val password: String
)

/**
 * POST /api/v1/rooms/:id/verify-password 成功响应 data 字段（T-30038）
 *
 * @param accessToken 访问令牌，用于 WS JoinRoom 消息鉴权
 */
data class VerifyPasswordResponseData(
    @SerializedName("access_token") val accessToken: String
)
