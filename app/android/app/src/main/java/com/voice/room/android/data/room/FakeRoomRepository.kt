package com.voice.room.android.data.room

import androidx.paging.PagingSource
import androidx.paging.PagingState
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import java.io.IOException

/**
 * [IRoomRepository] 的测试 Fake 实现
 *
 * 预设 2 条房间数据，可通过属性控制响应行为：
 * - [shouldFail] = true 时所有方法返回失败
 * - [rooms] 可替换为任意房间列表
 * - [total] 可覆盖 total 字段（用于测试 hasMore 逻辑）
 * - [createdRoomId] 控制 createRoom 成功时返回的 room ID（T-30007）
 * - [verifyPasswordResult] 控制 verifyPassword 的返回结果（T-30038）
 */
open class FakeRoomRepository : IRoomRepository {

    var shouldFail = false

    var rooms: List<RoomItem> = listOf(
        RoomItem(
            roomId = "id-1",
            title = "房间A",
            roomType = "normal",
            memberCount = 5,
            maxMembers = 20,
            ownerNickname = "User1",
            ownerAvatar = null,
            createdAt = "2024-01-01T00:00:00Z"
        ),
        RoomItem(
            roomId = "id-2",
            title = "房间B",
            roomType = "password",
            memberCount = 10,
            maxMembers = 20,
            ownerNickname = "User2",
            ownerAvatar = "https://example.com/a.jpg",
            createdAt = "2024-01-01T00:00:00Z"
        )
    )

    var total: Int = 2

    /** T-30007: createRoom 成功时返回的 roomId */
    var createdRoomId: String = "fake-room-id"

    /**
     * T-30038: verifyPassword 的预设返回结果
     *
     * 默认成功返回 "fake-access-token"；
     * 测试中可替换为具体错误 Result.failure(PasswordWrongException(N))
     */
    var verifyPasswordResult: Result<String> = Result.success("fake-access-token")

    override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> {
        if (shouldFail) return Result.failure(IOException("Network error"))
        return Result.success(
            RoomsPage(
                total = total,
                page = page,
                items = rooms
            )
        )
    }

    /**
     * T-30006: 返回简单测试用 PagingSource，直接返回 [rooms] 列表（单页）
     * 若 [shouldFail]=true 则返回 [LoadResult.Error]
     */
    override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> {
        return object : PagingSource<Int, RoomItem>() {
            override fun getRefreshKey(state: PagingState<Int, RoomItem>): Int? = null

            override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> {
                if (shouldFail) {
                    return LoadResult.Error(IOException("Network error"))
                }
                return LoadResult.Page(
                    data = rooms,
                    prevKey = null,
                    nextKey = null
                )
            }
        }
    }

    /**
     * T-30007 + T-30036: 创建房间
     *
     * [shouldFail]=true 时返回 [Result.failure(IOException)]；
     * 否则返回 [Result.success(createdRoomId)]
     */
    override suspend fun createRoom(
        title: String,
        type: String,
        password: String?,
        coverUrl: String,
        category: String,
        announcement: String?
    ): Result<String> {
        if (shouldFail) return Result.failure(IOException("Network error"))
        return Result.success(createdRoomId)
    }

    /**
     * T-30038: 验证密码房密码
     *
     * 返回 [verifyPasswordResult] 预设值。
     * 测试中可覆盖该方法以实现永久挂起（模拟 Verifying 状态）。
     */
    override suspend fun verifyPassword(roomId: String, password: String): Result<String> =
        verifyPasswordResult
}
