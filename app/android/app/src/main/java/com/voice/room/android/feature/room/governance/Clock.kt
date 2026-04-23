package com.voice.room.android.feature.room.governance

/**
 * 可注入的时钟接口（T-30042）
 *
 * 通过依赖注入解耦 [System.currentTimeMillis]，使单元测试可精确控制时间。
 *
 * 生产代码使用 [SystemClock]；单元测试注入 FakeClock。
 */
interface Clock {
    /** 返回当前 epoch 时间（毫秒） */
    fun currentTimeMillis(): Long
}

/**
 * 系统时钟实现，委托给 [System.currentTimeMillis]
 */
class SystemClock : Clock {
    override fun currentTimeMillis(): Long = System.currentTimeMillis()
}
