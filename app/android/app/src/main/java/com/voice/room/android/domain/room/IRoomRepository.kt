package com.voice.room.android.domain.room

import androidx.paging.PagingSource

/**
 * 房间分页结果
 *
 * @param total 总房间数
 * @param page  当前页码（从 1 开始）
 * @param items 当前页房间列表
 */
data class RoomsPage(
    val total: Int,
    val page: Int,
    val items: List<RoomItem>
)

/**
 * 房间仓库领域接口
 *
 * 实现：[com.voice.room.android.data.room.RetrofitRoomRepository]
 * 测试：[com.voice.room.android.data.room.FakeRoomRepository]
 */
interface IRoomRepository {
    /**
     * 获取房间列表（分页）
     *
     * @param page 页码（从 1 开始）
     * @param size 每页条数
     * @return [Result.success] 包含 [RoomsPage]；网络 / 业务异常时返回 [Result.failure]
     */
    suspend fun getRooms(page: Int, size: Int): Result<RoomsPage>

    /**
     * 返回用于 Paging3 无限滚动的 [PagingSource]（T-30006 新增）
     *
     * 生产实现：[com.voice.room.android.data.room.RoomPagingSource]
     * 测试实现：FakeRoomRepository 内联匿名 PagingSource
     */
    fun getRoomsPagingSource(): PagingSource<Int, RoomItem>

    /**
     * 创建新房间（T-30007 新增）
     *
     * 对应 protocol.md §3.1 POST /api/v1/rooms
     *
     * @param title    房间标题（1–30 Unicode 字符；服务端再校验）
     * @param type     房间类型：`normal` / `password` / `paid`
     * @param password 密码（`type=password` 时必填；其他类型传 null）
     * @return [Result.success] 包含新建房间 ID；校验失败或 API 错误返回 [Result.failure]
     */
    suspend fun createRoom(title: String, type: String, password: String?): Result<String>
}
