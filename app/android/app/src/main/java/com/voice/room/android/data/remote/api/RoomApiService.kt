package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.CreateRoomRequest
import com.voice.room.android.data.remote.model.CreateRoomResponseData
import com.voice.room.android.data.remote.model.RoomListResponseData
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.Query

/**
 * Retrofit 房间接口定义（对应 protocol.md §3.1 和 §3.2）
 */
interface RoomApiService {

    /**
     * 获取房间列表（第一页大厅）
     *
     * GET /api/v1/rooms?page=1&size=20
     * 无需鉴权 — 公开接口
     *
     * @param page 页码（从 1 开始）
     * @param size 每页条数（默认 20）
     */
    @GET("rooms")
    suspend fun getRooms(
        @Query("page") page: Int,
        @Query("size") size: Int
    ): Response<ApiResponse<RoomListResponseData>>

    /**
     * 创建房间 (T-30007)
     *
     * POST /api/v1/rooms
     * 需要 JWT 鉴权（由 AuthInterceptor 注入 Authorization 头）
     *
     * 成功返回 HTTP 201；失败参见 protocol.md §3.1 Error Scenarios
     *
     * @param request 包含 title、room_type、password 的请求体
     */
    @POST("rooms")
    suspend fun createRoom(
        @Body request: CreateRoomRequest
    ): Response<ApiResponse<CreateRoomResponseData>>
}
