package com.voice.room.android.core.media

/**
 * 测试用 [IMediaService] Fake 实现（T-30013）
 *
 * 记录所有方法调用，供单元测试断言验证。
 * 通过各 `*Result` 属性可注入任意失败场景（R1 LOW fix）。
 *
 * 用法示例：
 * ```kotlin
 * val fake = FakeMediaService()
 * fake.joinChannelResult = Result.failure(RuntimeException("RTC error"))
 * fake.startPublishAudioResult = Result.failure(RuntimeException("publish failed"))
 * fake.stopPublishAudioResult = Result.failure(RuntimeException("stop failed"))
 * fake.leaveChannelResult = Result.failure(RuntimeException("leave failed"))
 *
 * // 调用后验证：
 * assertEquals(1, fake.joinChannelCalls.size)
 * assertEquals("room-1" to "me", fake.joinChannelCalls[0])
 * assertEquals(1, fake.startPublishAudioCalls.size)
 * ```
 */
class FakeMediaService : IMediaService {

    /** 记录每次 [joinChannel] 调用的 roomId to userId 对 */
    val joinChannelCalls = mutableListOf<Pair<String, String>>()

    /** 记录每次 [leaveChannel] 调用次数 */
    val leaveChannelCalls = mutableListOf<Unit>()

    /** 记录每次 [startPublishAudio] 调用次数 */
    val startPublishAudioCalls = mutableListOf<Unit>()

    /** 记录每次 [stopPublishAudio] 调用次数 */
    val stopPublishAudioCalls = mutableListOf<Unit>()

    /** 可注入 joinChannel 的返回结果，默认成功 */
    var joinChannelResult: Result<Unit> = Result.success(Unit)

    /** 可注入 startPublishAudio 的返回结果，默认成功（R1 LOW fix） */
    var startPublishAudioResult: Result<Unit> = Result.success(Unit)

    /** 可注入 stopPublishAudio 的返回结果，默认成功（R1 LOW fix） */
    var stopPublishAudioResult: Result<Unit> = Result.success(Unit)

    /** 可注入 leaveChannel 的返回结果，默认成功（R1 LOW fix） */
    var leaveChannelResult: Result<Unit> = Result.success(Unit)

    override fun providerName(): String = "fake"

    override suspend fun joinChannel(roomId: String, userId: String): Result<Unit> {
        joinChannelCalls.add(roomId to userId)
        return joinChannelResult
    }

    override suspend fun leaveChannel(): Result<Unit> {
        leaveChannelCalls.add(Unit)
        return leaveChannelResult
    }

    override suspend fun startPublishAudio(): Result<Unit> {
        startPublishAudioCalls.add(Unit)
        return startPublishAudioResult
    }

    override suspend fun stopPublishAudio(): Result<Unit> {
        stopPublishAudioCalls.add(Unit)
        return stopPublishAudioResult
    }
}
