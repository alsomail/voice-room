package com.voice.room.android.core.network

/**
 * 401 未授权事件处理契约（Domain 层接口）
 *
 * 当 HTTP 响应码为 401 时，[AuthInterceptor] 调用 [onUnauthorized]。
 * 生产实现：[DefaultUnauthorizedHandler]（清除 Token + 发射导航事件）
 * 测试中注入 Fake 实现，无任何 Android 框架依赖。
 */
interface UnauthorizedHandler {
    /**
     * 处理 401 响应：
     * - 清除本地 JWT Token
     * - 发射 "跳转到登录页" 导航事件
     *
     * 并发安全：实现须保证多线程并发调用时只触发一次登出流程。
     */
    suspend fun onUnauthorized()

    /**
     * 重置"已处理"标记，使下一次 401 可再次触发登出流程。
     *
     * **调用时机**：用户重新登录成功（[com.voice.room.android.domain.local.ITokenManager.saveToken]
     * 调用完毕）后调用，确保下一次 Token 失效仍能正常触发登出。
     */
    fun resetUnauthorized()
}
