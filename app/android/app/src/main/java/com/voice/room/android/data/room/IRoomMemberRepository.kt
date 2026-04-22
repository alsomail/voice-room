package com.voice.room.android.data.room

import com.voice.room.android.data.model.RoomMember

/**
 * 房间成员列表仓库接口（T-30039）
 *
 * 抽象分页获取房间成员列表的 HTTP 请求，便于单元测试替换为 Fake。
 *
 * 实现类：
 * - `RetrofitRoomMemberRepository`（生产）
 * - `FakeRoomMemberRepository`（测试）
 */
interface IRoomMemberRepository {

    /**
     * 获取指定房间的成员列表（分页）。
     *
     * @param roomId 目标房间 ID
     * @param page   页码（从 1 开始）
     * @param limit  每页条数
     * @return 成员列表分页结果
     * @throws Exception HTTP 请求失败或服务端返回错误时抛出
     */
    suspend fun listMembers(roomId: String, page: Int, limit: Int): MemberListResult
}

/**
 * 成员列表分页结果
 *
 * @param members 本页成员列表
 * @param total   服务端成员总数
 * @param hasMore 是否还有更多页
 */
data class MemberListResult(
    val members: List<RoomMember>,
    val total: Int,
    val hasMore: Boolean,
)

/**
 * 空实现，生产环境中由 [RetrofitRoomMemberRepository] 替换，
 * 测试中由 [FakeRoomMemberRepository] 替换。
 *
 * Review R1 HIGH-02 修复：`hasMore = false`（原为 true），避免在未接入 DI 场景下
 * 每次 `loadMoreMembers()` 都成功返回并导致 `currentPage` 无限累积。
 */
class NoOpRoomMemberRepository : IRoomMemberRepository {
    override suspend fun listMembers(roomId: String, page: Int, limit: Int): MemberListResult =
        MemberListResult(members = emptyList(), total = 0, hasMore = false)
}
