package com.voice.room.android.domain.user

/**
 * 用户资料领域模型（Domain 层，与 DTO 解耦）
 *
 * 对应 protocol.md §2.3 GET /api/v1/users/me 响应中的 data 字段。
 *
 * @param id          服务端分配的用户 UUID
 * @param phone       完整手机号（含国家码，如 "+966512345678"）
 * @param nickname    用户昵称
 * @param avatar      头像 URL（可为 null，表示用户未设置头像）
 * @param coinBalance 金币余额（Long 保证大整数安全）
 * @param vipLevel    VIP 等级（0 = 普通用户）
 * @param createdAt   账号创建时间（ISO 8601 字符串，如 "2026-04-17T00:00:00Z"）
 */
data class UserProfile(
    val id: String,
    val phone: String,
    val nickname: String,
    val avatar: String?,
    val coinBalance: Long,
    val vipLevel: Int,
    val createdAt: String
)
