package com.voice.room.android.data.room

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.RoomApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.CreateRoomRequest
import com.voice.room.android.data.remote.model.RoomItemDto
import com.voice.room.android.data.remote.model.VerifyPasswordRequest
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.PasswordLockedException
import com.voice.room.android.domain.room.PasswordWrongException
import com.voice.room.android.domain.room.RoomNotFoundException
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
 * - T-30038: [verifyPassword] → POST /api/v1/rooms/:id/verify-password
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
     * T-30007 + T-30036: 创建房间
     *
     * POST /api/v1/rooms → 成功返回新房间 ID；
     * HTTP 4xx/5xx 或 code≠0 → [Result.failure(ApiException)]
     */
    override suspend fun createRoom(
        title: String,
        type: String,
        password: String?,
        coverUrl: String,
        category: String,
        announcement: String?
    ): Result<String> =
        runCatching {
            val request = CreateRoomRequest(
                title = title,
                roomType = type,
                password = password,
                coverUrl = coverUrl.ifBlank { null },
                category = category.ifBlank { null },
                announcement = announcement?.ifBlank { null }
            )
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

    /**
     * T-30038: 验证密码房密码
     *
     * POST /api/v1/rooms/:id/verify-password
     * - 成功 (code=0) → Result.success(access_token)
     * - 40103 → [PasswordWrongException](remainingAttempts)
     * - 42910 → [PasswordLockedException](remainingMinutes)
     * - 40400 → [RoomNotFoundException]
     * - 其它 → [ApiException]
     */
    override suspend fun verifyPassword(roomId: String, password: String): Result<String> =
        runCatching {
            val request = VerifyPasswordRequest(password = password)
            val response = api.verifyPassword(roomId, request)

            if (!response.isSuccessful) {
                val errJson = response.errorBody()?.string()
                if (!errJson.isNullOrBlank()) {
                    runCatching {
                        val errType = object : TypeToken<ApiResponse<Map<String, Any>>>() {}.type
                        val errBody: ApiResponse<Map<String, Any>> =
                            gson.fromJson(errJson, errType)
                        when (errBody.code) {
                            40103 -> {
                                val remaining = (errBody.data?.get("remaining_attempts") as? Double)
                                    ?.toInt() ?: 1
                                throw PasswordWrongException(remaining)
                            }
                            42910 -> {
                                // 缺陷 #1 修复：服务端返回字段为 `locked_remaining_sec`（秒），
                                // 之前误读为 `remaining_minutes` 且当 minutes 处理 → 锁定时长被压成 1/60。
                                // 默认值同步对齐 LOCK_TTL_SECS = 1800 秒（= 30 分钟）。
                                val secs = (errBody.data?.get("locked_remaining_sec") as? Double)
                                    ?.toInt() ?: 1800
                                throw PasswordLockedException(secs)
                            }
                            40400 -> throw RoomNotFoundException()
                            else  -> throw ApiException(errBody.code, errBody.message)
                        }
                    }.onSuccess { /* unreachable */ }
                        .onFailure { throw it }
                }
                throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
            }

            val body = response.body()
                ?: throw ApiException(50001, "Empty response body")
            if (body.code != 0) throw ApiException(body.code, body.message)
            val data = body.data
                ?: throw ApiException(50001, "Null data in response")

            data.accessToken
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
