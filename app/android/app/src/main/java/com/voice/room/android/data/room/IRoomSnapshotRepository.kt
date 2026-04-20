package com.voice.room.android.data.room

/**
 * 房间快照仓库接口（T-30010）
 *
 * 抽象进入房间时的 HTTP 请求，隔离网络实现，便于单元测试替换为 Fake。
 *
 * 实现类：
 * - `RetrofitRoomSnapshotRepository`（生产）
 * - `FakeRoomSnapshotRepository`（测试，定义于 RoomViewModelTest.kt）
 */
interface IRoomSnapshotRepository {

    /**
     * 获取指定房间的初始快照（麦位状态、在线人数、房间名等）。
     *
     * @param roomId 目标房间 ID
     * @return 房间快照数据
     * @throws Exception HTTP 请求失败或服务端返回错误时抛出
     */
    suspend fun getRoomSnapshot(roomId: String): RoomSnapshot
}
