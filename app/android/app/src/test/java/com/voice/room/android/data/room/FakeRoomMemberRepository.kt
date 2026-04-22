package com.voice.room.android.data.room

import com.voice.room.android.data.model.RoomMember

/**
 * [IRoomMemberRepository] 的测试 Fake 实现（T-30039）
 *
 * 默认返回空结果（hasMore = true），可通过 [result] 字段设置自定义返回值，
 * 通过 [throwError] 注入异常模拟网络错误。
 */
class FakeRoomMemberRepository : IRoomMemberRepository {

    /** 下次 [listMembers] 调用时返回的结果，默认空列表 + hasMore=true */
    var result: MemberListResult = MemberListResult(
        members = emptyList(),
        total = 0,
        hasMore = true,
    )

    /** 设置后，下次 [listMembers] 调用时抛出该异常 */
    var throwError: Exception? = null

    /** 记录所有调用的参数，便于断言验证 */
    val calls = mutableListOf<Triple<String, Int, Int>>()

    override suspend fun listMembers(roomId: String, page: Int, limit: Int): MemberListResult {
        calls.add(Triple(roomId, page, limit))
        throwError?.let { throw it }
        return result
    }
}
