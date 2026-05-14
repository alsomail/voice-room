package com.voice.room.android.feature.noble

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.core.theme.MenaColors
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive

/** Entrance queue entry */
data class NobleEntrance(
    val userId: String,
    val tierLevel: Int,
    val tierName: String,
    val badgeColor: String
)

/**
 * NobleEntrancePlayer — 进场特效播放器 (T-30072)
 *
 * FIFO 队列处理多个进场动画，防止叠加爆炸。
 * URL 由 Server 下发，不硬编码。
 */
@Composable
fun NobleEntrancePlayer(
    entrance: NobleEntrance?,
    modifier: Modifier = Modifier
) {
    AnimatedVisibility(
        visible = entrance != null,
        enter = slideInVertically() + fadeIn(),
        exit = slideOutVertically() + fadeOut(),
        modifier = modifier
    ) {
        entrance?.let { e ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(8.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(
                        NOBLE_COLORS.getOrElse(e.tierLevel - 1) { NOBLE_COLORS[0] }
                            .copy(alpha = 0.2f)
                    )
                    .testTag("noble_entrance_${e.userId}")
                    .padding(12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.Center
            ) {
                NobleBadge(tierLevel = e.tierLevel, userId = e.userId)
                Spacer(Modifier.width(8.dp))
                Column {
                    Text(
                        e.tierName,
                        fontWeight = FontWeight.Bold,
                        fontSize = 14.sp,
                        color = MenaColors.Primary
                    )
                    Text(
                        "entered the room",
                        fontSize = 12.sp,
                        color = Color.White.copy(alpha = 0.7f)
                    )
                }
            }
        }
    }
}

/**
 * FIFO 队列控制器 — 按顺序播放进场动画 (T-30072 #3)
 */
class EntranceQueueController {
    private val queue = ArrayDeque<NobleEntrance>()
    private var isPlaying = false

    suspend fun enqueue(entrance: NobleEntrance, onPlay: suspend (NobleEntrance) -> Unit) {
        queue.addLast(entrance)
        if (!isPlaying) playNext(onPlay)
    }

    private suspend fun playNext(onPlay: suspend (NobleEntrance) -> Unit) {
        if (queue.isEmpty()) { isPlaying = false; return }
        isPlaying = true
        val next = queue.removeFirst()
        onPlay(next)
        delay(3000) // Show for 3 seconds
        playNext(onPlay)
    }
}
