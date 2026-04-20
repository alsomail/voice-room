package com.voice.room.android.feature.room

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomsPage
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
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
    private val roomRepository: IRoomRepository = NoOpRoomRepository
) : ViewModel() {

    companion object {
        const val PAGE_SIZE = 20
    }

    private val _uiState = MutableStateFlow(HallUiState())
    val uiState: StateFlow<HallUiState> = _uiState.asStateFlow()

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
                            error = "网络异常，请稍后重试",
                            rooms = emptyList()
                        )
                    }
                }
        }
    }

    /**
     * 下拉刷新：重置列表并从第 1 页重新加载
     */
    fun refresh() {
        _uiState.update { it.copy(rooms = emptyList()) }
        loadRooms(page = 1)
    }

    // ─────────────────────────────────────────────
    // Factory（生产环境依赖注入）
    // ─────────────────────────────────────────────

    class Factory(
        private val roomRepository: IRoomRepository
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            HallViewModel(roomRepository) as T
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
            password: String?
        ): Result<String> =
            Result.failure(
                IllegalStateException("No IRoomRepository injected. Use HallViewModel.Factory.")
            )
    }
}
