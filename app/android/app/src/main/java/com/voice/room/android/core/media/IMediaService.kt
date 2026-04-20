package com.voice.room.android.core.media

/**
 * 媒体服务防腐层接口（T-30013）
 *
 * 隔离真实 RTC SDK（Agora/Zego），使 ViewModel 层可在不依赖具体 SDK 的情况下测试。
 * MVP 阶段由 [NoOpMediaService] 实现；测试注入 [FakeMediaService]。
 */
interface IMediaService {
    fun providerName(): String

    /**
     * 加入 RTC 频道（上麦成功后调用）
     * @param roomId 房间 ID，对应 RTC 频道名
     * @param userId 当前用户 ID
     */
    suspend fun joinChannel(roomId: String, userId: String): Result<Unit>

    /** 离开 RTC 频道（下麦成功后调用） */
    suspend fun leaveChannel(): Result<Unit>

    /** 开始推送音频流（加入频道后调用） */
    suspend fun startPublishAudio(): Result<Unit>

    /** 停止推送音频流（离开频道前调用） */
    suspend fun stopPublishAudio(): Result<Unit>
}
