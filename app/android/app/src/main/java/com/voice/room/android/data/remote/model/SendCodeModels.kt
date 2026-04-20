package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/** POST /api/v1/auth/verification-codes 请求体 */
data class SendCodeRequest(
    @SerializedName("phone") val phone: String
)

/** POST /api/v1/auth/verification-codes 成功响应 data 字段 */
data class SendCodeResponseData(
    @SerializedName("expires_in") val expiresIn: Int,
    @SerializedName("cooldown") val cooldown: Int
)
