package com.voice.room.android.domain.auth

/**
 * 登录成功后的领域结果模型（与 DTO 解耦）
 *
 * @param token   JWT 字符串，需持久化到 DataStore
 * @param userId  Server 分配的用户 UUID
 * @param isNew   首次注册时为 true，可用于展示新手引导
 */
data class LoginResult(
    val token: String,
    val userId: String,
    val isNew: Boolean
)

/**
 * 发送验证码成功后的领域结果模型
 *
 * @param cooldownSeconds  服务端返回的冷却倒计时秒数（协议约定 60s）
 */
data class SendCodeResult(
    val cooldownSeconds: Int
)
