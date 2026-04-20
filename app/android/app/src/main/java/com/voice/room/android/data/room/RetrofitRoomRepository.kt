package com.voice.room.android.data.room

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.RoomApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.CreateRoomRequest
import com.voice.room.android.data.remote.model.RoomItemDto
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage

/**
 * [IRoomRepository] 的 Retrofit 真实实现
 *
 * - 通过 [RoomApiService] 发起 HTTP 请求
 * - HTTP 2xx + code==0 → 映射 DTO 到领域模型
 * - HTTP 4xx/5xx → 解析 error body 为 [ApiException]
 * - 业务 code ≠ 0 → 抛出 [ApiException]
 * - 网络异常 → 原样封装为 [Result.failure]
 * - T-30006: [getRoomsPagingSource] → 委托 [RoomPagingSource]
 * - T-30007: [createRoom] → POST /api/v1/rooms
 */
class RetrofitRoomRepository(
    private val api: RoomApiService
) : IRoomRepository {

    private val gson = Gson()

    override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
        runCatching {
            val response = api.getRooms(page, size)

            if (!response.isSuccessful) {
                val errJson = response.errorBody()?.string()
                if (!errJson.isNullOrBlank()) {
                    runCatching {
                        val type = object : TypeToken<ApiResponse<Nothing>>() {}.type
                        val errBody: ApiResponse<Nothing> = gson.fromJson(errJson, type)
                        throw ApiException(errBody.code, errBody.message)
                    }.onSuccess { /* unreachable — throw above */ }
                        .onFailure { if (it is ApiException) throw it }
                }
                throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
            }

            val body = response.body()
                ?: throw ApiException(50001, "Empty response body")
            if (body.code != 0) throw ApiException(body.code, body.message)
            val data = body.data
                ?: throw ApiException(50001, "Null data in response")

            RoomsPage(
                total = data.total,
                page = data.page,
                items = data.items.map { it.toDomain() }
            )
        }

    /** T-30006: 返回委托本仓库的 Paging3 数据源 */
    override fun getRoomsPagingSource(): androidx.paging.PagingSource<Int, RoomItem> =
        RoomPagingSource(this)

    /**
     * T-30007: 创建房间
     *
     * POST /api/v1/rooms → 成功返回新房间 ID；
     * HTTP 4xx/5xx 或 code≠0 → [Result.failure(ApiException)]
     */
    override suspend fun createRoom(
        title: String,
        type: String,
        password: String?
    ): Result<String> =
        runCatching {
            val request = CreateRoomRequest(title = title, roomType = type, password = password)
            val response = api.createRoom(request)

            if (!response.isSuccessful) {
                val errJson = response.errorBody()?.string()
                if (!errJson.isNullOrBlank()) {
                    runCatching {
                        val errType = object : TypeToken<ApiResponse<Nothing>>() {}.type
                        val errBody: ApiResponse<Nothing> = gson.fromJson(errJson, errType)
                        throw ApiException(errBody.code, errBody.message)
                    }.onSuccess { /* unreachable */ }
                        .onFailure { if (it is ApiException) throw it }
                }
                throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
            }

            val body = response.body()
                ?: throw ApiException(50001, "Empty response body")
            if (body.code != 0) throw ApiException(body.code, body.message)
            val data = body.data
                ?: throw ApiException(50001, "Null data in response")

            data.roomId
        }
}

// ─────────────────────────────────────────────
// Extension: DTO → 领域模型
// ─────────────────────────────────────────────

internal fun RoomItemDto.toDomain(): RoomItem = RoomItem(
    roomId = roomId,
    title = title,
    roomType = roomType,
    memberCount = memberCount,
    maxMembers = maxMembers,
    ownerNickname = ownerNickname,
    ownerAvatar = ownerAvatar,
    createdAt = createdAt
)
