package com.voice.room.android.feature.room.governance

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 禁麦/禁言倒计时 ViewModel（T-30042）
 *
 * 维护当前用户的麦克风禁用（micExpiresAt）和聊天禁用（chatExpiresAt）到期时间戳。
 * mic 与 chat 相互独立，互不影响。
 *
 * ### 使用方式
 * - RoomViewModel 收到 `UserMuted` WS 事件时调用 [startMicCountdown] / [startChatCountdown]
 * - `duration_sec=0` 时调用 [clearMic] / [clearChat]
 * - UI 层读取 [micRemainingSeconds] / [chatRemainingSeconds] 展示倒计时
 *
 * @param clock 时钟接口，生产使用 [SystemClock]，测试注入 FakeClock
 */
class MuteCountdownViewModel(
    private val clock: Clock = SystemClock()
) : ViewModel() {

    // ─── 麦克风禁用 ────────────────────────────────────────────────────────────

    private val _micExpiresAt = MutableStateFlow<Long?>(null)

    /**
     * 禁麦到期时间戳（epoch 毫秒）；null 表示未被禁麦。
     * 连续两次调用取最新值（直接覆盖）。
     */
    val micExpiresAt: StateFlow<Long?> = _micExpiresAt.asStateFlow()

    // ─── 聊天禁用 ──────────────────────────────────────────────────────────────

    private val _chatExpiresAt = MutableStateFlow<Long?>(null)

    /**
     * 禁言到期时间戳（epoch 毫秒）；null 表示未被禁言。
     * 连续两次调用取最新值（直接覆盖）。
     */
    val chatExpiresAt: StateFlow<Long?> = _chatExpiresAt.asStateFlow()

    // ─── 公开操作 ──────────────────────────────────────────────────────────────

    /**
     * 开始禁麦倒计时。
     *
     * 若已有进行中的禁麦倒计时，直接覆盖为新的 [expiresAt]（取最新值）。
     *
     * @param expiresAt 禁麦到期时间戳（epoch 毫秒）
     */
    fun startMicCountdown(expiresAt: Long) {
        _micExpiresAt.value = expiresAt
    }

    /**
     * 开始禁言倒计时。
     *
     * 若已有进行中的禁言倒计时，直接覆盖为新的 [expiresAt]（取最新值）。
     *
     * @param expiresAt 禁言到期时间戳（epoch 毫秒）
     */
    fun startChatCountdown(expiresAt: Long) {
        _chatExpiresAt.value = expiresAt
    }

    /**
     * 清除禁麦状态（解除禁麦，duration_sec=0 时调用）。
     * 将 [micExpiresAt] 置为 null，UI 倒计时 Chip 自动消失。
     */
    fun clearMic() {
        _micExpiresAt.value = null
    }

    /**
     * 清除禁言状态（解除禁言，duration_sec=0 时调用）。
     * 将 [chatExpiresAt] 置为 null，UI 倒计时 Chip 自动消失。
     */
    fun clearChat() {
        _chatExpiresAt.value = null
    }

    // ─── 剩余时间计算 ──────────────────────────────────────────────────────────

    /**
     * 计算禁麦剩余秒数。
     *
     * @return 剩余秒数；若已到期或未禁麦则返回 0
     */
    fun micRemainingSeconds(): Long {
        val exp = _micExpiresAt.value ?: return 0L
        return ((exp - clock.currentTimeMillis()) / 1000L).coerceAtLeast(0L)
    }

    /**
     * 计算禁言剩余秒数。
     *
     * @return 剩余秒数；若已到期或未禁言则返回 0
     */
    fun chatRemainingSeconds(): Long {
        val exp = _chatExpiresAt.value ?: return 0L
        return ((exp - clock.currentTimeMillis()) / 1000L).coerceAtLeast(0L)
    }
}
