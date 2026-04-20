package com.voice.room.android.feature.room

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.paging.Pager
import androidx.paging.PagingConfig
import androidx.paging.PagingData
import androidx.paging.cachedIn
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import kotlinx.coroutines.flow.Flow

/**
 * 房间列表 Paging3 ViewModel (T-30006)
 *
 * 通过 [Pager] + [IRoomRepository.getRoomsPagingSource] 构建无限滚动数据流，
 * 使用 [cachedIn] 在 [viewModelScope] 中缓存，避免旋转屏等配置变更时重复加载。
 *
 * @param repository 房间仓库（生产: [RetrofitRoomRepository]，测试: [FakeRoomRepository]）
 */
class RoomListViewModel(
    private val repository: IRoomRepository = FakeRoomRepository()
) : ViewModel() {

    /**
     * 分页数据流 — 每次 collect 得到的是最新一页 [PagingData]，
     * UI 层调用 [collectAsLazyPagingItems] 渲染列表。
     *
     * 注意：显式设置 [PagingConfig.initialLoadSizeHint] = [pageSize] = 20，
     * 覆盖 Paging3 默认的 pageSize×3=60，确保首次 Refresh 与后续 Append
     * 使用相同的 loadSize=20，彻底消除数据重叠问题（T-30006 Review Round 1 修复）。
     */
    val pagingFlow: Flow<PagingData<RoomItem>> = Pager(
        config = PagingConfig(
            pageSize = 20,
            initialLoadSize = 20,  // 防止默认 3×pageSize=60 导致 Refresh 与 Append 数据重叠
            enablePlaceholders = false,
            prefetchDistance = 5
        )
    ) {
        repository.getRoomsPagingSource()
    }.flow.cachedIn(viewModelScope)

    // ─────────────────────────────────────────────
    // Factory（生产环境依赖注入）
    // ─────────────────────────────────────────────

    class Factory(
        private val repository: IRoomRepository
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T =
            RoomListViewModel(repository) as T
    }
}
