package com.voice.room.android.feature.splash

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.consent.ConsentRepository
import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * SplashViewModel — JWT 检测 + 隐私同意首启动弹窗
 *
 * 职责（R1 批 2 缺陷 5）：
 * 1. 启动时通过 [ConsentRepository] 加载已保存的同意状态
 *    - 已设置 → 直接走 [checkAuth] 流程
 *    - 未设置 → 暴露 [showConsent] = true，UI 渲染 [com.voice.room.android.core.consent.PrivacyConsentDialog]
 * 2. 用户在弹窗中选择 → [onConsentSelected] 持久化模式 → 关闭弹窗 → 触发 [checkAuth]
 * 3. JWT 检测：读取本地 Token，发射 [SplashNavEvent] 通知 UI 导航
 *
 * ViewModel 不持有 NavController 引用，仅通过 SharedFlow + StateFlow 暴露给 UI 层。
 */
class SplashViewModel(
    private val tokenManager: ITokenManager,
    private val consentRepository: ConsentRepository? = null,
) : ViewModel() {

    private val _navEvent = MutableSharedFlow<SplashNavEvent>()
    val navEvent: SharedFlow<SplashNavEvent> = _navEvent.asSharedFlow()

    private val _showConsent = MutableStateFlow(false)
    /** 是否需要显示隐私同意弹窗（首次启动且 ConsentRepository 未持久化时） */
    val showConsent: StateFlow<Boolean> = _showConsent.asStateFlow()

    /**
     * Splash 启动时调用：先 load consent，未设置则弹窗，否则直接 checkAuth。
     */
    fun bootstrap() {
        viewModelScope.launch {
            val repo = consentRepository
            if (repo != null) {
                runCatching { repo.load() }
                if (!repo.isSet) {
                    _showConsent.value = true
                    return@launch
                }
            }
            checkAuth()
        }
    }

    /**
     * 用户在隐私弹窗中选择后调用，持久化并继续 [checkAuth]。
     */
    fun onConsentSelected(mode: ConsentMode) {
        viewModelScope.launch {
            consentRepository?.saveConsent(mode)
            _showConsent.value = false
            checkAuth()
        }
    }

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
                throw e
            } catch (_: Exception) {
                null
            }
            if (token != null && token.isNotBlank()) {
                _navEvent.emit(SplashNavEvent.NavigateToMain)
            } else {
                _navEvent.emit(SplashNavEvent.NavigateToLogin)
            }
        }
    }

    class Factory(
        private val tokenManager: ITokenManager,
        private val consentRepository: ConsentRepository? = null,
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return SplashViewModel(tokenManager, consentRepository) as T
        }
    }
}
