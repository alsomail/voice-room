package com.voice.room.android.feature.room.governance

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.R
import kotlinx.coroutines.delay

/**
 * 禁麦/禁言倒计时 Chip（T-30042）
 *
 * - 每秒刷新剩余时间显示
 * - 剩余时间归零时调用 [onExpired]
 * - testTag: `mute_countdown_mic`（麦克风类型）, `mute_countdown_chat`（聊天类型）
 *
 * @param muteType       禁用类型："mic"（禁麦）或 "chat"（禁言）
 * @param expiresAtMs    到期时间戳（epoch 毫秒）
 * @param clock          时钟接口（生产使用 SystemClock，测试注入 FakeClock）
 * @param onExpired      倒计时归零回调
 */
@Composable
fun MuteStatusChip(
    muteType: String,
    expiresAtMs: Long,
    clock: Clock = SystemClock(),
    onExpired: () -> Unit = {},
) {
    val tag = if (muteType == "mic") "mute_countdown_mic" else "mute_countdown_chat"

    var remainingSec by remember {
        mutableLongStateOf(
            ((expiresAtMs - clock.currentTimeMillis()) / 1000L).coerceAtLeast(0L)
        )
    }

    // 每秒更新倒计时
    LaunchedEffect(expiresAtMs) {
        while (remainingSec > 0L) {
            delay(1_000L)
            remainingSec = ((expiresAtMs - clock.currentTimeMillis()) / 1000L).coerceAtLeast(0L)
        }
        onExpired()
    }

    val minutes = remainingSec / 60
    val seconds = remainingSec % 60
    val timeText = "%d:%02d".format(minutes, seconds)
    val labelFormatRes = if (muteType == "mic") {
        R.string.room_governance_mute_chip_mic_format
    } else {
        R.string.room_governance_mute_chip_chat_format
    }

    Row(
        modifier = Modifier
            .background(
                color = Color(0xCC000000),
                shape = RoundedCornerShape(12.dp),
            )
            .padding(horizontal = 10.dp, vertical = 4.dp)
            .testTag(tag),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = stringResource(labelFormatRes, timeText),
            color = Color(0xFFFF6B6B),
            fontSize = 12.sp,
        )
    }
}
