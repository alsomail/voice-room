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
     * 创建新房间（T-30007 + T-30036）
     *
     * 对应 protocol.md §3.1 POST /api/v1/rooms
     *
     * @param title        房间标题（1–30 Unicode 字符；服务端再校验）
     * @param type         房间类型：`normal` / `password` / `paid`
     * @param password     密码（`type=password` 时必填；其他类型传 null）
     * @param coverUrl     封面图 URL（T-30036 新增）
     * @param category     房间分类 key（T-30036 新增）：chat / emotion / music / game / matchmaking / other
     * @param announcement 公告（T-30036 新增，可选，最多 200 字符）
     * @return [Result.success] 包含新建房间 ID；校验失败或 API 错误返回 [Result.failure]
     */
    suspend fun createRoom(
        title: String,
        type: String,
        password: String?,
        coverUrl: String = "",
        category: String = "",
        announcement: String? = null
    ): Result<String>

    /**
     * 验证密码房密码（T-30038）
     *
     * 对应 POST /api/v1/rooms/:id/verify-password
     *
     * @param roomId   目标房间 ID
     * @param password 用户输入的密码
     * @return [Result.success] 包含 access_token（用于后续 WS JoinRoom）
     * @throws [PasswordWrongException] 密码错误（HTTP 40103），含剩余次数
     * @throws [PasswordLockedException] 已被锁定（HTTP 42910），含剩余分钟
     * @throws [RoomNotFoundException] 房间不存在（HTTP 40400）
     */
    suspend fun verifyPassword(roomId: String, password: String): Result<String>
}
