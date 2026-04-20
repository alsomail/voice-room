package com.voice.room.android.data.room

import androidx.paging.PagingSource
import androidx.paging.PagingState
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem

/**
 * Paging3 数据源 — 委托 [IRoomRepository.getRooms] 实现逐页加载
 *
 * - Key 类型：[Int]（页码，从 1 开始）
 * - Value 类型：[RoomItem]
 * - loadSize 上限 100 条，防止接口超载
 *
 * nextKey 计算：total > page * size → 还有下一页
 * prevKey 计算：page > 1 → 有上一页
 */
class RoomPagingSource(
    private val repository: IRoomRepository
) : PagingSource<Int, RoomItem>() {

    /**
     * 根据当前 anchor 位置恢复页码（下拉刷新或配置变更后使用）
     */
    override fun getRefreshKey(state: PagingState<Int, RoomItem>): Int? =
        state.anchorPosition?.let { anchor ->
            state.closestPageToPosition(anchor)?.run {
                prevKey?.plus(1) ?: nextKey?.minus(1)
            }
        }

    /**
     * 加载一页数据
     *
     * @param params [LoadParams.Refresh] key=null 时从第 1 页开始；
     *               [LoadParams.Append] key 为下一页页码；
     *               [LoadParams.Prepend] 暂不支持（不会触发）
     */
    override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> {
        val page = params.key ?: 1
        val size = params.loadSize.coerceAtMost(100)
        return repository.getRooms(page, size).fold(
            onSuccess = { data ->
                LoadResult.Page(
                    data = data.items,
                    prevKey = if (page > 1) page - 1 else null,
                    nextKey = if (data.total > page * size) page + 1 else null
                )
            },
            onFailure = { error ->
                LoadResult.Error(error)
            }
        )
    }
}
