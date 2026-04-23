package com.voice.room.android.feature.room.governance

/**
 * 当前用户自身的禁麦/禁言治理状态（T-30044）
 *
 * 与 [MuteCountdownViewModel] 互补：
 * - [MuteCountdownViewModel] 驱动倒计时 UI 展示（剩余秒数）
 * - [SelfGovernanceState] 控制 UI 操作置灰（是否允许上麦/发言）
 *
 * 使用 [Long] 时间戳而非 [java.time.Instant]，便于注入 [Clock] 进行测试。
 *
 * @param micMutedUntil  禁麦到期时间戳（epoch 毫秒）；null 表示未被禁麦
 * @param chatMutedUntil 禁言到期时间戳（epoch 毫秒）；null 表示未被禁言
 */
data class SelfGovernanceState(
    val micMutedUntil: Long? = null,
    val chatMutedUntil: Long? = null,
) {
    /**
     * 是否处于禁麦状态。
     *
     * @param nowMs 当前时间戳（epoch 毫秒），由 [Clock.currentTimeMillis] 提供
     * @return true = 禁麦中（nowMs < micMutedUntil），false = 未被禁麦或已到期
     */
    fun isMicMuted(nowMs: Long): Boolean =
        micMutedUntil != null && nowMs < micMutedUntil

    /**
     * 是否处于禁言状态。
     *
     * @param nowMs 当前时间戳（epoch 毫秒），由 [Clock.currentTimeMillis] 提供
     * @return true = 禁言中（nowMs < chatMutedUntil），false = 未被禁言或已到期
     */
    fun isChatMuted(nowMs: Long): Boolean =
        chatMutedUntil != null && nowMs < chatMutedUntil
}
