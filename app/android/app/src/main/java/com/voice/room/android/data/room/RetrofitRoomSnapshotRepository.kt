package com.voice.room.android.data.room

import com.voice.room.android.data.remote.api.RoomApiService

/**
 * [IRoomSnapshotRepository] 的 Retrofit 生产实现（BUG-ROOM-NAV 修复）
 *
 * 调用 `GET /api/v1/rooms/{id}` 接口获取房间详情，映射为 [RoomSnapshot] 供 [com.voice.room.android.feature.room.RoomViewModel] 使用。
 *
 * 麦位映射说明：
 * - MVP 阶段服务端 mic_slots 固定返回空数组（参见 dto.rs 注释）
 * - 此处返回空 micSlots 列表，由 [com.voice.room.android.feature.room.RoomViewModel.toRoomUiState] 补全 9 个空麦位
 *
 * @param roomApiService Retrofit 房间 API 接口（由 [com.voice.room.android.common.AppContainer] 注入）
 */
class RetrofitRoomSnapshotRepository(
    private val roomApiService: RoomApiService
) : IRoomSnapshotRepository {

    override suspend fun getRoomSnapshot(roomId: String): RoomSnapshot {
        val response = roomApiService.getRoomDetail(roomId)
        val body = response.body()
            ?: throw Exception("getRoomDetail: empty HTTP body (HTTP ${response.code()}) for room $roomId")
        val data = body.data
            ?: throw Exception("getRoomDetail: null data payload for room $roomId — server msg: ${body.message}")

        return RoomSnapshot(
            roomId = data.roomId,
            roomName = data.title,
            onlineCount = data.memberCount,
            micSlots = emptyList(),   // MVP: server returns empty array; RoomViewModel pads to 9 slots
        )
    }
}
