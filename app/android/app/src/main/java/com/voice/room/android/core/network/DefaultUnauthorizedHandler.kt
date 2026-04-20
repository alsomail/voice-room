package com.voice.room.android.core.network

import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import java.util.concurrent.atomic.AtomicBoolean

/**
 * [UnauthorizedHandler] 的默认实现。
 *
 * 当收到 HTTP 401 时：
 * 1. 调用 [ITokenManager.clearToken] 清除本地 JWT Token
 * 2. 向 [unauthorizedEvent] 发射 [Unit]，供 UI/ViewModel 层监听并跳转登录页
 *
 * ## 并发安全（H-01 修复）
 * 使用 [AtomicBoolean] 实现 compare-and-set 一次性语义：多个 OkHttp 后台线程并发触发 401 时，
 * 只有第一个 `compareAndSet(false, true)` 成功的调用会执行登出逻辑，其余调用立即返回。
 * 用户重新登录成功后（`saveToken` 完成），调用 [resetUnauthorized] 重置标记，
 * 以确保下次 Token 失效时登出流程仍能正常触发。
 *
 * ## SharedFlow 配置
 * 使用 [extraBufferCapacity] = 1 + [BufferOverflow.DROP_OLDEST] 确保
 * 来自 OkHttp 后台线程的 [kotlinx.coroutines.runBlocking] 调用不会永远挂起。
 *
 * ## 单例要求（M-01）
 * ⚠️ **必须以应用级单例注入**（Hilt `@Singleton` 或 Koin `single { }`）。
 * [AuthInterceptor] 与 UI 层必须共享同一实例，否则事件通道断开，401 无法触达 UI。
 *
 * 接入示例（ViewModel 中监听）：
 * ```kotlin
 * unauthorizedHandler.unauthorizedEvent.collect {
 *     navController.navigate("login") { popUpTo(0) { inclusive = true } }
 * }
 * ```
 */
class DefaultUnauthorizedHandler(
    private val tokenManager: ITokenManager
) : UnauthorizedHandler {

    /** 原子标记：确保并发 401 只触发一次登出流程。 */
    private val handled = AtomicBoolean(false)

    private val _unauthorizedEvent = MutableSharedFlow<Unit>(
        replay = 0,
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST
    )

    /**
     * 订阅此 Flow 以监听 "跳转登录页" 导航事件。
     * replay = 0，每次事件只被消费一次。
     */
    val unauthorizedEvent: SharedFlow<Unit> = _unauthorizedEvent.asSharedFlow()

    /**
     * 处理 401：原子保证只执行一次。
     * 后续并发调用在 `compareAndSet` 处直接返回，不重复清 Token 或发射事件。
     */
    override suspend fun onUnauthorized() {
        if (!handled.compareAndSet(false, true)) return  // 并发保护：只有第一次通过
        tokenManager.clearToken()          // 先清除 Token
        _unauthorizedEvent.tryEmit(Unit)   // 再发射导航事件（非挂起，不阻塞 OkHttp 线程）
    }

    /**
     * 重置"已处理"标记。
     *
     * 用户重新登录成功（[ITokenManager.saveToken] 完成）后调用，
     * 使下一次 Token 失效时 [onUnauthorized] 能再次触发完整登出流程。
     */
    override fun resetUnauthorized() {
        handled.set(false)
    }
}
