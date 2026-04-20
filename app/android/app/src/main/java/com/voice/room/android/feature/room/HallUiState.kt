package com.voice.room.android.feature.room

import com.voice.room.android.domain.room.RoomItem

/**
 * 大厅页 UI 状态（不可变 data class）
 *
 * @param rooms        当前页房间列表
 * @param isLoading    是否正在加载
 * @param error        错误提示文本（null 表示无错误）
 * @param currentPage  当前已加载的页码
 * @param totalItems   服务端房间总数
 * @param hasMore      是否还有更多数据（= totalItems > currentPage * PAGE_SIZE）
 */
data class HallUiState(
    val rooms: List<RoomItem> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
    val currentPage: Int = 1,
    val totalItems: Int = 0,
    val hasMore: Boolean = false
) {
    companion object {
        const val PAGE_SIZE = 20
    }
}
