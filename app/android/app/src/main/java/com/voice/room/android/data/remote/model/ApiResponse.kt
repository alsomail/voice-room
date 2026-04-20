package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 统一 API 响应包装体（匹配 protocol.md §1.3）
 *
 * 成功：code=0, data≠null
 * 失败：code≠0, data=null, message 为错误描述
 */
data class ApiResponse<T>(
    @SerializedName("code") val code: Int,
    @SerializedName("message") val message: String,
    @SerializedName("data") val data: T?,
    @SerializedName("request_id") val requestId: String?
)
