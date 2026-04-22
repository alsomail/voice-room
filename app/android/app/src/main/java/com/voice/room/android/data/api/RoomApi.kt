package com.voice.room.android.data.api

import com.voice.room.android.data.remote.model.ApiResponse
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.Path
import retrofit2.http.Query

/**
 * 房间成员列表 Retrofit 接口（T-30039）
 *
 * GET /api/v1/rooms/:roomId/members?page=1&limit=20
 * 需要 JWT 鉴权（由 AuthInterceptor 注入 Authorization 头）
 */
interface RoomApi {

    /**
     * 获取房间成员列表（分页）
     *
     * @param roomId 目标房间 ID（路径参数）
     * @param page   页码（从 1 开始，默认 1）
     * @param limit  每页条数（默认 20）
     */
    @GET("rooms/{roomId}/members")
    suspend fun listMembers(
        @Path("roomId") roomId: String,
        @Query("page") page: Int = 1,
        @Query("limit") limit: Int = 20,
    ): Response<ApiResponse<MemberListResponseData>>
}

/**
 * 成员列表接口响应数据（T-30039）
 */
data class MemberListResponseData(
    val members: List<MemberDto>,
    val total: Int,
    val hasMore: Boolean,
)

/**
 * 单个成员 DTO（服务端字段映射）
 */
data class MemberDto(
    val id: String,
    val nickname: String,
    val avatarUrl: String? = null,
    val role: String = "member",
    val slot: Int? = null,
    val joinedAt: Long = 0L,
    val micMuted: Boolean = false,
    val chatMuted: Boolean = false,
)
