package com.voice.room.android.feature.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.R
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.user.UserProfile
import com.voice.room.android.util.UiText
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * ProfileViewModel — 个人中心 ViewModel (T-30024)
 *
 * 负责：
 * - 初始化时调用 [loadProfile] 加载用户资料
 * - 网络异常时降级使用 in-memory 缓存
 * - [logout]：清除 JWT token 并发射 [ProfileEvent.NavigateToLogin]
 * - [copyId]：发射 [ProfileEvent.ShowToast] 通知 UI 层执行剪贴板写入
 *
 * 结构化并发：CancellationException 必须 re-throw，不得吞噬。
 */
class ProfileViewModel(
    private val userRepository: IUserRepository,
    private val tokenManager: ITokenManager,
) : ViewModel() {

    private val _uiState = MutableStateFlow<ProfileUiState>(ProfileUiState.Loading)
    val uiState: StateFlow<ProfileUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<ProfileEvent>()
    val events: SharedFlow<ProfileEvent> = _events.asSharedFlow()

    /** in-memory 缓存（ViewModel 生命周期内有效）：网络异常时降级使用 */
    private var cachedProfile: UserProfile? = null

    init {
        loadProfile()
    }

    /**
     * 加载用户资料。
     *
     * 成功 → [ProfileUiState.Success]（fromCache=false）+ 更新 in-memory 缓存
     * IOException → 有缓存：[ProfileUiState.Success]（fromCache=true）+ [ProfileEvent.ShowToast]
     *             → 无缓存：[ProfileUiState.Error]
     * CancellationException → 必须 re-throw（不能被 catch 吞噬）
     */
    fun loadProfile() {
        viewModelScope.launch {
            _uiState.value = ProfileUiState.Loading
            userRepository.getMe()
                .onSuccess { profile ->
                    cachedProfile = profile
                    _uiState.value = ProfileUiState.Success(profile, fromCache = false)
                }
                .onFailure { e ->
                    if (e is CancellationException) throw e  // 结构化并发：必须 re-throw
                    val cached = cachedProfile
                    if (cached != null) {
                        _uiState.value = ProfileUiState.Success(cached, fromCache = true)
                        // 缺陷 #2 修复：使用 UiText（@StringRes）替代中文字面量
                        _events.emit(
                            ProfileEvent.ShowToast(UiText.of(R.string.profile_cached_data_toast))
                        )
                    } else {
                        // 错误信息使用底层异常 message（IOException 等通常已是英文/技术消息）；
                        // 当 message 为 null/blank 时，UI 层 ProfileErrorContent 会回退到
                        // R.string.profile_load_failed（缺陷 #2）
                        _uiState.value = ProfileUiState.Error(e.message.orEmpty())
                    }
                }
        }
    }

    /**
     * 复制用户 ID 到剪贴板（由 ProfileScreen 执行实际写入，ViewModel 仅发射 Toast 事件）。
     *
     * @param userId 要复制的用户 ID 字符串
     */
    fun copyId(userId: String) {
        viewModelScope.launch {
            // 缺陷 #2 修复：UiText 占位，UI 层按 Locale 解析
            _events.emit(ProfileEvent.ShowToast(UiText.of(R.string.profile_id_copied_toast)))
        }
    }

    /**
     * 退出登录：清除 JWT token → 发射 [ProfileEvent.NavigateToLogin]。
     *
     * 在 Loading / Success / Error 任意状态下均可安全调用。
     */
    fun logout() {
        viewModelScope.launch {
            tokenManager.clearToken()
            _events.emit(ProfileEvent.NavigateToLogin)
        }
    }

    companion object {
        /**
         * 工厂方法，供 [viewModel(factory = ...)] 注入依赖使用。
         */
        fun factory(
            userRepository: IUserRepository,
            tokenManager: ITokenManager,
        ) = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T =
                ProfileViewModel(userRepository, tokenManager) as T
        }
    }
}
