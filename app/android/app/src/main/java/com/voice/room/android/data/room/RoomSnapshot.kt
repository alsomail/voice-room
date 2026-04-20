package com.voice.room.android.data.room

/**
 * 进入房间 HTTP 快照响应（T-30010）
 *
 * 由 [IRoomSnapshotRepository.getRoomSnapshot] 返回，供 [RoomViewModel] 初始化 UI 状态。
 */
data class RoomSnapshot(
    val roomId: String,
    val roomName: String,
    val onlineCount: Int,
    /** 服务端返回的当前麦位列表（可能不足 9 个，ViewModel 负责补全空麦位） */
    val micSlots: List<MicSlotData>,
)

/**
 * 单个麦位原始数据
 *
 * @param index    0–8，对应 9 宫格位置
 * @param userId   null 表示空麦
 * @param nickname 用户昵称
 */
data class MicSlotData(
    val index: Int,
    val userId: String?,
    val nickname: String?,
)
