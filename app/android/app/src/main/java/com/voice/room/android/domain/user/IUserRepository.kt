package com.voice.room.android.domain.user

/**
 * 用户信息仓库接口（Domain 层契约）
 *
 * - 所有方法均为 suspend，供 ViewModel 的 viewModelScope.launch 调用
 * - 返回 [Result<T>]，调用方通过 onSuccess / onFailure 处理结果
 * - 失败时异常类型由实现层决定（如 [com.voice.room.android.data.auth.ApiException]）
 */
interface IUserRepository {

    /**
     * 获取当前登录用户的资料信息
     *
     * 调用 GET /api/v1/users/me（需要 JWT 认证，由 AuthInterceptor 自动注入）
     *
     * @return [Result.success] 包含 [UserProfile]；
     *         [Result.failure] 包含 [com.voice.room.android.data.auth.ApiException]（4xx/5xx 业务错误）
     *         或 [java.io.IOException]（网络错误）
     */
    suspend fun getMe(): Result<UserProfile>
}
