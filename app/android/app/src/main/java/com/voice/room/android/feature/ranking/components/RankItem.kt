package com.voice.room.android.feature.ranking.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
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
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.domain.ranking.RankEntry

/** 金色光圈（Top1） */
private val GoldColor = Color(0xFFAFA14B)

/** 银色光圈（Top2） */
private val SilverColor = Color(0xFFC0C0C0)

/** 铜色光圈（Top3） */
private val BronzeColor = Color(0xFFCD7F32)

/**
 * 榜单条目组件 (T-30033)
 *
 * - rank 1: 王冠 Icon + 金色(AFA14B)边框头像
 * - rank 2: 银色(C0C0C0)边框
 * - rank 3: 铜色(CD7F32)边框
 * - rank 4+: 数字排名，普通头像（无边框）
 *
 * testTag: `rank_item_{rank}`
 */
@Composable
fun RankItem(
    entry: RankEntry,
    modifier: Modifier = Modifier,
) {
    val borderColor: Color? = when (entry.rank) {
        1 -> GoldColor
        2 -> SilverColor
        3 -> BronzeColor
        else -> null
    }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .testTag("rank_item_${entry.rank}")
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // ── 排名 / 王冠 ────────────────────────────────────────
        if (entry.rank == 1) {
            Icon(
                imageVector = Icons.Filled.Star,
                contentDescription = "王冠",
                tint = GoldColor,
                modifier = Modifier.size(24.dp),
            )
        } else {
            Text(
                text = entry.rank.toString(),
                color = MenaColors.OnBackgroundSecondary,
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.width(24.dp),
            )
        }

        Spacer(modifier = Modifier.width(12.dp))

        // ── 头像（带光圈） ──────────────────────────────────────
        Box(
            modifier = Modifier
                .size(44.dp)
                .clip(CircleShape)
                .then(
                    if (borderColor != null) Modifier.border(2.dp, borderColor, CircleShape)
                    else Modifier
                )
                .background(MenaColors.SurfaceVariant),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = entry.nickname.take(1),
                color = MenaColors.OnBackground,
                fontSize = 16.sp,
            )
        }

        Spacer(modifier = Modifier.width(12.dp))

        // ── 昵称 ────────────────────────────────────────────────
        Text(
            text = entry.nickname,
            color = MenaColors.OnBackground,
            fontSize = 15.sp,
            modifier = Modifier.weight(1f),
        )

        // ── 分数 ────────────────────────────────────────────────
        Text(
            text = "%,d".format(entry.score),
            color = MenaColors.Primary,
            fontSize = 14.sp,
            fontWeight = FontWeight.Bold,
        )
    }
}
