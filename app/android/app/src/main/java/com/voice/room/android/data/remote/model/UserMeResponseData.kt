package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * GET /api/v1/users/me 成功响应中的 data 字段 DTO
 *
 * 字段映射对应 protocol.md §2.3 users/me 响应结构：
 * ```json
 * {
 *   "id": "550e8400-e29b-41d4-a716-446655440000",
 *   "phone": "+966512345678",
 *   "nickname": "User_a1b2",
 *   "avatar": "https://cdn.example.com/avatars/xxx.jpg",
 *   "coin_balance": 1000,
 *   "vip_level": 2,
 *   "created_at": "2026-04-17T00:00:00Z"
 * }
 * ```
 */
data class UserMeResponseData(
    @SerializedName("id") val id: String,
    @SerializedName("phone") val phone: String,
    @SerializedName("nickname") val nickname: String,
    @SerializedName("avatar") val avatar: String?,
    @SerializedName("coin_balance") val coinBalance: Long,
    @SerializedName("vip_level") val vipLevel: Int,
    @SerializedName("created_at") val createdAt: String
)
