package com.voice.room.android.domain.auth

/**
 * 认证仓库接口（Domain 层契约）
 *
 * - 所有方法均为 suspend，供 [viewModelScope.launch] 调用
 * - 返回 [Result<T>]，调用方通过 onSuccess / onFailure 处理结果
 * - 失败时异常类型由实现层决定（如 [com.voice.room.android.data.auth.ApiException]）
 */
interface IAuthRepository {

    /**
     * 发送短信验证码到指定手机号
     *
     * @param phone 完整手机号（含国家码，如 "+966501234567"）
     */
    suspend fun sendCode(phone: String): Result<SendCodeResult>

    /**
     * 手机号 + 验证码一步登录（手机号不存在时自动注册）
     *
     * @param phone 完整手机号（含国家码）
     * @param code  6 位短信验证码
     */
    suspend fun login(phone: String, code: String): Result<LoginResult>
}
