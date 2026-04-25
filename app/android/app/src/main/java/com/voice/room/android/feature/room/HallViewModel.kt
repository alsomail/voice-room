package com.voice.room.android.feature.room

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.R
import com.voice.room.android.data.local.InMemoryKickCooldownStore
import com.voice.room.android.data.local.KickCooldownStore
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.PasswordLockedException
import com.voice.room.android.domain.room.PasswordWrongException
import com.voice.room.android.domain.room.RoomNotFoundException
import com.voice.room.android.domain.room.RoomsPage
import com.voice.room.android.feature.room.governance.Clock
import com.voice.room.android.feature.room.governance.SystemClock
import com.voice.room.android.util.UiText
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * 大厅页 ViewModel
 *
 * - 构造时通过 [init] 自动加载第 1 页房间列表
 * - [loadRooms] 触发分页加载
 * - [refresh] 重置并重新加载第 1 页
 * - 所有状态通过 [uiState] 暴露，外部只读
 *
 * @param roomRepository 房间仓库（生产: [RetrofitRoomRepository]，测试: [FakeRoomRepository]）
 */
class HallViewModel(
    private val roomRepository: IRoomRepository = NoOpRoomRepository,
    private val kickCooldownStore: KickCooldownStore = InMemoryKickCooldownStore(),
    private val clock: Clock = SystemClock(),
) : ViewModel() {

    companion object {
        const val PAGE_SIZE = 20
    }

    private val _uiState = MutableStateFlow(HallUiState())
    val uiState: StateFlow<HallUiState> = _uiState.asStateFlow()

    // ─────────────────────────────────────────────
    // 密码弹窗状态 (T-30038)
    // ─────────────────────────────────────────────

    /** 密码弹窗状态；null 表示弹窗已关闭 */
    private val _passwordDialogState = MutableStateFlow<PasswordDialogState?>(null)
    val passwordDialogState: StateFlow<PasswordDialogState?> = _passwordDialogState.asStateFlow()

    /** 一次性 UI 事件（导航、Toast），由 Channel 保证不丢失 */
    private val _hallEvents = Channel<HallEvent>(Channel.UNLIMITED)
    val hallEvents: Flow<HallEvent> = _hallEvents.receiveAsFlow()

    /** 当前正在验证密码的房间 ID */
    private var passwordDialogRoomId: String? = null

    init {
        loadRooms(page = 1)
    }

    /**
     * 加载指定页的房间列表
     *
     * @param page 页码（从 1 开始）
     */
    fun loadRooms(page: Int) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, error = null) }
            roomRepository.getRooms(page, PAGE_SIZE)
                .onSuccess { result: RoomsPage ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            rooms = result.items,
                            totalItems = result.total,
                            currentPage = result.page,
                            hasMore = result.total > result.page * PAGE_SIZE
                        )
                    }
                }
                .onFailure {
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            // 缺陷 #4：使用 UiText（@StringRes）而非中文字面量
                            error = UiText.of(R.string.hall_load_failed),
                            rooms = emptyList()
                        )
                    }
                }
        }
    }

    /**
     * 下拉刷新：重置列表 + 分页元数据，并从第 1 页重新加载。
     *
     * 缺陷 #5 修复：之前仅清空 [HallUiState.rooms]，未重置
     * [HallUiState.currentPage]、[HallUiState.totalItems]、[HallUiState.hasMore]，
     * 导致刷新后若新结果首页为空（或更短），分页器仍认为存在历史页 → 上拉无效 / 状态错乱。
     */
    fun refresh() {
        _uiState.update {
            it.copy(
                rooms = emptyList(),
                currentPage = 1,
                totalItems = 0,
                hasMore = false,
                error = null,
            )
        }
        loadRooms(page = 1)
    }

    // ─────────────────────────────────────────────
    // 密码弹窗操作 (T-30038)
    // ─────────────────────────────────────────────

    /**
     * 打开密码输入弹窗
     *
     * @param roomId 目标密码房 ID
     */
    fun openPasswordDialog(roomId: String) {
        passwordDialogRoomId = roomId
        _passwordDialogState.value = PasswordDialogState.Idle
    }

    /**
     * 关闭密码弹窗（返回键 / 点击外部区域）
     *
     * 不进入房间，重置弹窗状态为 null。
     */
    fun dismissPasswordDialog() {
        passwordDialogRoomId = null
        _passwordDialogState.value = null
    }

    /**
     * 提交密码验证（6 位输完自动调用）
     *
     * 流程：
     * 1. 立即切换状态为 [PasswordDialogState.Verifying]
     * 2. 调用 [IRoomRepository.verifyPassword]
     * 3. 成功 → 发出 [HallEvent.NavigateToRoom]，关闭弹窗
     * 4. 40103 → 切 [PasswordDialogState.Error]
     * 5. 42910 → 切 [PasswordDialogState.Locked]
     * 6. 40400 → Toast "房间不存在" + 关闭弹窗
     * 7. 其它 → Toast "网络错误，请重试"
     *
     * @param password 用户输入的 6 位密码
     */
    fun verifyPassword(password: String) {
        val roomId = passwordDialogRoomId ?: return
        _passwordDialogState.value = PasswordDialogState.Verifying
        viewModelScope.launch {
            roomRepository.verifyPassword(roomId, password)
                .onSuccess { accessToken ->
                    _hallEvents.trySend(HallEvent.NavigateToRoom(roomId, accessToken))
                    _passwordDialogState.value = null
                }
                .onFailure { error ->
                    when (error) {
                        is PasswordWrongException ->
                            _passwordDialogState.value =
                                PasswordDialogState.Error(error.remainingAttempts)

                        is PasswordLockedException ->
                            _passwordDialogState.value =
                                // 缺陷 #1：服务端契约字段为秒数 → state 也用秒
                                PasswordDialogState.Locked(error.remainingSeconds)

                        is RoomNotFoundException -> {
                            // 缺陷 #4：使用 UiText 占位
                            _hallEvents.trySend(HallEvent.ShowToast(UiText.of(R.string.hall_room_not_found)))
                            _passwordDialogState.value = null
                        }

                        else ->
                            _hallEvents.trySend(
                                HallEvent.ShowToast(UiText.of(R.string.hall_password_unknown_error))
                            )
                    }
                }
        }
    }

    // ─────────────────────────────────────────────
    // 进房检查（T-30042）
    // ─────────────────────────────────────────────

    /**
     * 普通房进入入口（T-30042）。
     *
     * 进房前检查 kickCooldown：
     * - 若 cooldown 未过期 → 发出 [HallEvent.ShowToast] 并阻止进房
     * - 若 cooldown 已过期或无记录 → 发出 [HallEvent.NavigateToRoom]
     *
     * @param roomId 目标房间 ID
     */
    fun enterRoom(roomId: String) {
        val untilMs = kickCooldownStore.get(roomId)
        val nowMs = clock.currentTimeMillis()
        if (untilMs > nowMs) {
            val remainingSec = ((untilMs - nowMs) / 1000L).coerceAtLeast(1L)
            // 缺陷 #4：使用 UiText 占位（i18n），消息体在 UI 层格式化
            _hallEvents.trySend(
                HallEvent.ShowToast(
                    UiText.of(R.string.hall_kick_cooldown_seconds, remainingSec.toInt())
                )
            )
            return
        }
        _hallEvents.trySend(HallEvent.NavigateToRoom(roomId, accessToken = null))
    }

    // ─────────────────────────────────────────────
    // Factory（生产环境依赖注入）
    // ─────────────────────────────────────────────

    class Factory(
        private val roomRepository: IRoomRepository,
        private val kickCooldownStore: KickCooldownStore = InMemoryKickCooldownStore(),
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            HallViewModel(roomRepository, kickCooldownStore) as T
    }

    // ─────────────────────────────────────────────
    // NoOp 默认实现（Preview 和无参构造时使用）
    // ─────────────────────────────────────────────

    private object NoOpRoomRepository : IRoomRepository {
        override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
            Result.failure(
                IllegalStateException("No IRoomRepository injected. Use HallViewModel.Factory.")
            )

        override fun getRoomsPagingSource(): androidx.paging.PagingSource<Int, com.voice.room.android.domain.room.RoomItem> =
            object : androidx.paging.PagingSource<Int, com.voice.room.android.domain.room.RoomItem>() {
                override fun getRefreshKey(
                    state: androidx.paging.PagingState<Int, com.voice.room.android.domain.room.RoomItem>
                ): Int? = null

                override suspend fun load(
                    params: LoadParams<Int>
                ): LoadResult<Int, com.voice.room.android.domain.room.RoomItem> =
                    LoadResult.Error(
                        IllegalStateException(
                            "No IRoomRepository injected. Use HallViewModel.Factory."
                        )
                    )
            }

        override suspend fun createRoom(
            title: String,
            type: String,
            password: String?,
            coverUrl: String,
            category: String,
            announcement: String?
        ): Result<String> =
            Result.failure(
                IllegalStateException("No IRoomRepository injected. Use HallViewModel.Factory.")
            )

        override suspend fun verifyPassword(roomId: String, password: String): Result<String> =
            Result.failure(
                IllegalStateException("No IRoomRepository injected. Use HallViewModel.Factory.")
            )
    }
}
