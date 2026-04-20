package com.voice.room.android.core.media

/**
 * 媒体服务空操作实现（MVP 占位）
 *
 * 所有操作均成功返回但不执行任何实际 RTC 操作。
 * 真实 RTC SDK 接入后由具体实现替换此类。
 */
class NoOpMediaService : IMediaService {
    override fun providerName(): String = "media-adapter-pending"
    override suspend fun joinChannel(roomId: String, userId: String): Result<Unit> = Result.success(Unit)
    override suspend fun leaveChannel(): Result<Unit> = Result.success(Unit)
    override suspend fun startPublishAudio(): Result<Unit> = Result.success(Unit)
    override suspend fun stopPublishAudio(): Result<Unit> = Result.success(Unit)
}
