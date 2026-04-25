package com.voice.room.android.feature.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.EventKey
import com.voice.room.android.core.analytics.impl.NoopAnalytics
import com.voice.room.android.core.network.UnauthorizedHandler
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.domain.auth.IAuthRepository
import com.voice.room.android.domain.auth.SendCodeResult
import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * 登录页 ViewModel – 负责所有业务逻辑，UI 只做纯渲染。
 *
 * ### 设计原则
 * - 所有状态通过 [uiState] 暴露，外部只读
 * - 导航事件通过 [navEvent]（SharedFlow）单次消费
 * - 倒计时与 API 调用均在 [viewModelScope] 内运行，ViewModel 销毁时自动取消
 * - 通过构造注入 [IAuthRepository] 和 [ITokenManager]，方便单元测试替换 Fake
 *
 * ### 依赖注入
 * 在 Composable 中通过 [viewModel] 工厂使用默认无参构造（NoOp 实现）；
 * 生产环境应通过 [LoginViewModel.Factory] 注入真实实现。
 *
 * @param authRepository      认证仓库（发送验证码 + 登录）
 * @param tokenManager        JWT Token 本地持久化
 * @param unauthorizedHandler 401 未授权处理器（登录成功后调用 resetUnauthorized 重置状态）
 */
class LoginViewModel(
    private val authRepository: IAuthRepository = NoOpAuthRepository,
    private val tokenManager: ITokenManager = NoOpTokenManager,
    private val unauthorizedHandler: UnauthorizedHandler = NoOpUnauthorizedHandler,
    /**
     * Analytics 防腐层（T-30035 / R1 批 2 缺陷 2）。
     * 业务层只能通过 [AnalyticsPort] 调用，严禁直接 import io.sentry.*；
     * 默认 [NoopAnalytics] 用于 Compose Preview 与无 DI 的单测路径。
     */
    private val analyticsPort: AnalyticsPort = NoopAnalytics()
) : ViewModel() {

    private val _uiState = MutableStateFlow(LoginUiState())
    val uiState: StateFlow<LoginUiState> = _uiState.asStateFlow()

    /**
     * 单次导航事件流（SharedFlow，replay=0）
     * 登录成功时发射 [NavEvent.NavigateToHall]。
     */
    private val _navEvent = MutableSharedFlow<NavEvent>()
    val navEvent: SharedFlow<NavEvent> = _navEvent.asSharedFlow()

    // ─────────────────────────────────────────────
    // 手机号输入
    // ─────────────────────────────────────────────

    /** 用户修改手机号输入框时调用（传入不含 +966 前缀的号码字符串）。 */
    fun onPhoneNumberChanged(phone: String) {
        _uiState.update { it.copy(phoneNumber = phone) }
    }

    // ─────────────────────────────────────────────
    // 验证码输入
    // ─────────────────────────────────────────────

    /** 用户修改验证码输入框时调用（最多 6 位数字）。 */
    fun onVerificationCodeChanged(code: String) {
        _uiState.update { it.copy(verificationCode = code) }
    }

    // ─────────────────────────────────────────────
    // 发送验证码（T-30002：真实 API 调用）
    // ─────────────────────────────────────────────

    /**
     * 点击"发送验证码"时调用：
     * 1. 防抖：发送按钮不可用时直接返回
     * 2. 置 isSendingCode=true，清空错误
     * 3. 调用 [IAuthRepository.sendCode]
     * 4. 成功 → 设置 countdownSeconds，启动倒计时协程
     * 5. 失败 → 展示友好错误信息
     */
    fun onSendCode() {
        if (!_uiState.value.isSendButtonEnabled) return

        viewModelScope.launch {
            _uiState.update { it.copy(isSendingCode = true, error = null) }

            val phone = "+966${_uiState.value.phoneNumber}"
            authRepository.sendCode(phone)
                .onSuccess { result ->
                    _uiState.update {
                        it.copy(
                            isSendingCode = false,
                            countdownSeconds = result.cooldownSeconds
                        )
                    }
                    startCountdown(result)
                }
                .onFailure { error ->
                    _uiState.update {
                        it.copy(
                            isSendingCode = false,
                            error = mapSendCodeError(error)
                        )
                    }
                }
        }
    }

    // ─────────────────────────────────────────────
    // 登录（T-30002：真实 API 调用）
    // ─────────────────────────────────────────────

    /**
     * 点击"登录"时调用：
     * 1. 防抖：登录按钮不可用时直接返回
     * 2. 置 isLoading=true，清空错误
     * 3. 调用 [IAuthRepository.login]
     * 4. 成功 → 保存 token → 置 isLoginSuccess=true → 发射 [NavEvent.NavigateToHall]
     * 5. 失败 → 展示友好错误信息，清除 isLoading
     */
    fun onLogin() {
        if (!_uiState.value.isLoginButtonEnabled) return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, error = null) }

            val phone = "+966${_uiState.value.phoneNumber}"
            val code = _uiState.value.verificationCode

            authRepository.login(phone, code)
                .onSuccess { result ->
                    runCatching { tokenManager.saveToken(result.token) }
                        .onSuccess {
                            unauthorizedHandler.resetUnauthorized()
                            // T-30035 R1 批 2（缺陷 2）：登录验证成功埋点。
                            // 公共字段（device_id/session_id/...）由 CommonPropsProvider 注入，
                            // 此处仅传业务字段 is_new_user（business_flows §2.9 字典对齐）。
                            analyticsPort.track(
                                EventKey.LOGIN_VERIFY_SUCCESS,
                                mapOf("is_new_user" to result.isNew)
                            )
                            _uiState.update {
                                it.copy(
                                    isLoading = false,
                                    isLoginSuccess = true,
                                    isNewUser = result.isNew
                                )
                            }
                            _navEvent.emit(NavEvent.NavigateToHall)
                        }
                        .onFailure {
                            _uiState.update {
                                it.copy(
                                    isLoading = false,
                                    error = "登录失败，Token 存储异常，请重试"
                                )
                            }
                        }
                }
                .onFailure { error ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            error = mapLoginError(error)
                        )
                    }
                }
        }
    }

    // ─────────────────────────────────────────────
    // Private helpers
    // ─────────────────────────────────────────────

    /** 启动倒计时协程（每秒递减 countdownSeconds） */
    private fun startCountdown(result: SendCodeResult) {
        viewModelScope.launch {
            repeat(result.cooldownSeconds) {
                delay(1_000L)
                _uiState.update { current ->
                    current.copy(
                        countdownSeconds = (current.countdownSeconds - 1).coerceAtLeast(0)
                    )
                }
            }
        }
    }

    /** 将登录接口异常映射为用户可读的中文错误信息 */
    private fun mapLoginError(error: Throwable): String = when {
        error is ApiException && error.code == 40103 -> "验证码错误"
        error is ApiException && error.code == 40104 -> "验证码已过期"
        error is ApiException && error.code == 40105 -> "验证码尝试次数超限"
        else -> "网络异常，请稍后重试"
    }

    /** 将发送验证码接口异常映射为用户可读的中文错误信息 */
    private fun mapSendCodeError(error: Throwable): String = when {
        error is ApiException && error.code == 40001 -> "手机号格式无效"
        error is ApiException && error.code == 42901 -> "发送过于频繁，请稍后再试"
        error is ApiException && error.code == 42902 -> "今日发送次数已超限"
        else -> "发送验证码失败，请稍后重试"
    }

    // ─────────────────────────────────────────────
    // Factory（生产环境依赖注入）
    // ─────────────────────────────────────────────

    class Factory(
        private val authRepository: IAuthRepository,
        private val tokenManager: ITokenManager,
        private val unauthorizedHandler: UnauthorizedHandler,
        private val analyticsPort: AnalyticsPort = NoopAnalytics()
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            LoginViewModel(authRepository, tokenManager, unauthorizedHandler, analyticsPort) as T
    }

    // ─────────────────────────────────────────────
    // NoOp 默认实现（用于 Compose Preview 和默认工厂）
    // ─────────────────────────────────────────────

    private object NoOpAuthRepository : IAuthRepository {
        override suspend fun sendCode(phone: String) =
            Result.success(com.voice.room.android.domain.auth.SendCodeResult(LoginUiState.COUNTDOWN_SECONDS))

        override suspend fun login(phone: String, code: String) =
            Result.failure<com.voice.room.android.domain.auth.LoginResult>(
                IllegalStateException("No real IAuthRepository injected. Use LoginViewModel.Factory.")
            )
    }

    private object NoOpTokenManager : ITokenManager {
        override suspend fun saveToken(token: String) = Unit
        override suspend fun getToken(): String? = null
        override suspend fun clearToken() = Unit
    }

    /**
     * NoOp 实现：用于默认参数及 Compose Preview，不执行任何操作。
     * 生产环境通过 [Factory] 注入真实的 [DefaultUnauthorizedHandler] 单例。
     */
    private object NoOpUnauthorizedHandler : UnauthorizedHandler {
        override suspend fun onUnauthorized() = Unit
        override fun resetUnauthorized() = Unit
    }
}
