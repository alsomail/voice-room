package com.voice.room.android.feature.noble

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Noble level → badge color mapping */
val NOBLE_COLORS = listOf(
    Color(0xFFFFD700), // Lv1 — Gold
    Color(0xFFFF8C00), // Lv2 — DarkOrange
    Color(0xFF9932CC), // Lv3 — Purple
    Color(0xFFFF1493), // Lv4 — DeepPink
    Color(0xFFFF0000), // Lv5 — Red
    Color(0xFFDC143C), // Lv6 — Crimson
)

/**
 * NobleBadge — 全局贵族徽章组件 (T-30073)
 *
 * 传 nobleTierLevel 渲染对应颜色徽章；level=null 时不占布局 (size=0)。
 * 在房间观众席/聊天消息/资料卡/麦位统一使用。
 */
@Composable
fun NobleBadge(
    tierLevel: Int?,
    userId: String,
    modifier: Modifier = Modifier
) {
    if (tierLevel == null || tierLevel < 1 || tierLevel > 6) return

    val color = NOBLE_COLORS.getOrElse(tierLevel - 1) { NOBLE_COLORS[0] }
    Box(
        modifier = modifier
            .size(18.dp)
            .clip(CircleShape)
            .background(color)
            .testTag("noble_badge_$userId"),
        contentAlignment = Alignment.Center
    ) {
        Text(
            "${tierLevel}",
            color = Color.White,
            fontSize = 10.sp,
            fontWeight = FontWeight.Bold
        )
    }
}

@Composable
fun NobleBadgeSmall(tierLevel: Int?, userId: String, modifier: Modifier = Modifier) {
    if (tierLevel == null || tierLevel < 1 || tierLevel > 6) return

    val color = NOBLE_COLORS.getOrElse(tierLevel - 1) { NOBLE_COLORS[0] }
    Box(
        modifier = modifier
            .size(14.dp)
            .clip(CircleShape)
            .background(color)
            .testTag("noble_badge_small_$userId"),
        contentAlignment = Alignment.Center
    ) {
        Text(
            "${tierLevel}",
            color = Color.White,
            fontSize = 8.sp,
            fontWeight = FontWeight.Bold
        )
    }
}
