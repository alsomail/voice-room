package com.voice.room.android.common.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * OnlineCountBadge — 在线人数徽标 (T-30022)
 *
 * 展示：绿色圆点(8dp, MenaColors.Success) + 在线人数文本(labelSmall, OnBackgroundSecondary)
 *
 * @param count 在线人数
 * @param modifier 可选修饰符
 */
@Composable
fun OnlineCountBadge(
    count: Int,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.testTag("online_count_badge"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        // 绿色圆点 8dp
        Box(
            modifier = Modifier
                .size(8.dp)
                .background(
                    color = MenaColors.Success,
                    shape = CircleShape,
                )
        )
        // 在线人数文字
        Text(
            text = "$count",
            style = MenaTypography.labelSmall,
            color = MenaColors.OnBackgroundSecondary,
        )
    }
}
