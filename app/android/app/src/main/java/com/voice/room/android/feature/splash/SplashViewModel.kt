package com.voice.room.android.feature.splash

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch

/**
 * SplashViewModel — JWT 检测逻辑
 *
 * 职责：读取本地 Token，判断有效性，发射导航事件。
 * ViewModel 不持有 NavController 引用，仅通过 SharedFlow 通知 UI 层。
 */
class SplashViewModel(
    private val tokenManager: ITokenManager
) : ViewModel() {

    private val _navEvent = MutableSharedFlow<SplashNavEvent>()
    val navEvent: SharedFlow<SplashNavEvent> = _navEvent.asSharedFlow()

    /**
     * 检查本地 JWT Token 有效性，发射对应导航事件。
     *
     * - token 非 null 且非空白 → [SplashNavEvent.NavigateToMain]
     * - token 为 null / 空 / 纯空白 / 读取异常 → [SplashNavEvent.NavigateToLogin]
     */
    fun checkAuth() {
        viewModelScope.launch {
            val token = try {
                tokenManager.getToken()
            } catch (e: CancellationException) {
                throw e  // 必须 rethrow，保持协程取消语义
            } catch (_: Exception) {
                null // DataStore 损坏等异常视为未登录
            }
            if (token != null && token.isNotBlank()) {
                _navEvent.emit(SplashNavEvent.NavigateToMain)
            } else {
                _navEvent.emit(SplashNavEvent.NavigateToLogin)
            }
        }
    }

    class Factory(
        private val tokenManager: ITokenManager
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return SplashViewModel(tokenManager) as T
        }
    }
}
