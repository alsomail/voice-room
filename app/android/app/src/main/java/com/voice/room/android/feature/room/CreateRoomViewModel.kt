package com.voice.room.android.feature.room

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.voice.room.android.VoiceRoomApplication
import com.voice.room.android.domain.room.IRoomRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * 创建房间对话框 ViewModel (T-30007)
 *
 * 职责：
 * 1. 维护 [uiState]（Idle → Loading → Success | Error）
 * 2. 客户端输入校验（标题长度、密码房校验）
 * 3. 幂等性保护：Loading 期间忽略重复提交
 *
 * 状态流转：
 * ```
 * Idle ──[createRoom]──> (validate) ──fail──> Error
 *                              │
 *                           pass
 *                              │
 *                           Loading ──[api]──> Success(roomId)
 *                                        └───> Error(message)
 * ```
 *
 * @param repository 房间仓库
 */
class CreateRoomViewModel(
    private val repository: IRoomRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow<CreateRoomUiState>(CreateRoomUiState.Idle)
    val uiState: StateFlow<CreateRoomUiState> = _uiState.asStateFlow()

    /**
     * 重置 UI 状态为 [CreateRoomUiState.Idle]。
     *
     * 在 BottomSheet 每次进入（Composition）时调用，确保上一次的 Success/Error 残留状态
     * 不会在重新打开时触发 [LaunchedEffect] 重放回调（重复导航问题）。
     */
    fun resetState() {
        _uiState.value = CreateRoomUiState.Idle
    }

    /**
     * 提交创建房间请求
     *
     * @param title    房间标题（1–30 Unicode 字符）
     * @param type     房间类型：`normal` / `password` / `paid`
     * @param password 密码（`type=password` 时必填）
     */
    fun createRoom(title: String, type: String, password: String? = null) {
        // 幂等性保护：Loading 时忽略重复调用
        if (_uiState.value is CreateRoomUiState.Loading) return

        // ── 客户端输入校验 ──────────────────────────
        val validationError = validate(title, type, password)
        if (validationError != null) {
            _uiState.value = CreateRoomUiState.Error(validationError)
            return
        }

        // ── 提交 API ────────────────────────────────
        viewModelScope.launch {
            _uiState.value = CreateRoomUiState.Loading

            repository.createRoom(title.trim(), type, password)
                .onSuccess { roomId ->
                    _uiState.value = CreateRoomUiState.Success(roomId)
                }
                .onFailure { error ->
                    _uiState.value = CreateRoomUiState.Error(
                        error.message?.takeIf { it.isNotBlank() } ?: "创建失败，请稍后重试"
                    )
                }
        }
    }

    // ─────────────────────────────────────────────
    // 输入校验
    // ─────────────────────────────────────────────

    /**
     * 客户端校验
     *
     * @return 校验错误文案（null 表示通过）
     */
    private fun validate(title: String, type: String, password: String?): String? {
        // 标题不能为空
        if (title.isBlank()) return "房间标题不能为空"

        // 标题不超过 30 个 Unicode 字符
        // codePointCount 精确计算 Unicode 字符数（含 emoji 等多码点字符）
        val charCount = title.codePointCount(0, title.length)
        if (charCount > MAX_TITLE_LENGTH) return "房间标题不能超过 $MAX_TITLE_LENGTH 个字符（当前 $charCount）"

        // 密码房必须提供密码
        if (type == ROOM_TYPE_PASSWORD && password.isNullOrBlank()) {
            return "密码房间必须设置密码"
        }

        return null
    }

    // ─────────────────────────────────────────────
    // Factory（生产环境注入）
    // ─────────────────────────────────────────────

    companion object {
        /** UI 层共享此常量，避免两处重复定义造成漂移风险（MEDIUM-01 修复）。 */
        internal const val MAX_TITLE_LENGTH = 30
        private const val ROOM_TYPE_PASSWORD = "password"

        /**
         * 生产 Factory — 通过 [VoiceRoomApplication.appContainer] 获取真实仓库。
         *
         * 用法（Fragment / Activity）：
         * ```kotlin
         * val vm: CreateRoomViewModel by viewModels { CreateRoomViewModel.Factory }
         * ```
         */
        val Factory: ViewModelProvider.Factory = viewModelFactory {
            initializer {
                val app = this[ViewModelProvider.AndroidViewModelFactory.APPLICATION_KEY]
                    as VoiceRoomApplication
                CreateRoomViewModel(app.appContainer.roomRepository)
            }
        }
    }
}
