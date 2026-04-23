package com.voice.room.android.utils

import com.voice.room.android.feature.room.governance.Clock

/**
 * 可注入的测试时钟，允许精确控制当前时间戳（T-30042）
 *
 * 使用示例：
 * ```kotlin
 * val fakeClock = FakeClock(currentTimeMs = 1_000_000L)
 * val vm = RoomViewModel(..., clock = fakeClock)
 * fakeClock.currentTimeMs += 600_000L  // 推进 10 分钟
 * ```
 */
class FakeClock(var currentTimeMs: Long = 0L) : Clock {
    override fun currentTimeMillis(): Long = currentTimeMs
}
