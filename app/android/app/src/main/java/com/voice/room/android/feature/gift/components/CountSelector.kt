package com.voice.room.android.feature.gift.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * 礼物数量档位选择器 (T-30028)
 *
 * 展示固定档位 [COUNT_OPTIONS] 的横向 Chip Row。
 * 选中项金色高亮（背景 + 文字），未选中项普通背景。
 *
 * testTag：每个 Chip 使用 `count_option_{value}`
 *
 * @param selectedCount 当前选中档位值
 * @param onCountSelected 用户点击档位回调，传出档位整数值
 * @param modifier 可选 Modifier
 */
@Composable
fun CountSelector(
    selectedCount: Int,
    onCountSelected: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        COUNT_OPTIONS.forEach { count ->
            val isSelected = count == selectedCount
            CountChip(
                count = count,
                isSelected = isSelected,
                onClick = { onCountSelected(count) },
            )
        }
    }
}

@Composable
private fun CountChip(
    count: Int,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    val backgroundColor = if (isSelected) MenaColors.Primary else MenaColors.Surface
    val contentColor = if (isSelected) MenaColors.Background else MenaColors.OnBackground
    val shape = RoundedCornerShape(16.dp)

    Surface(
        onClick = onClick,
        modifier = Modifier.testTag("count_option_$count"),
        shape = shape,
        color = backgroundColor,
        tonalElevation = if (isSelected) 0.dp else 2.dp,
    ) {
        Text(
            text = count.toString(),
            style = MenaTypography.labelMedium,
            color = contentColor,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        )
    }
}

/** 数量档位常量（TDS §方案设计 核心交互） */
val COUNT_OPTIONS = listOf(1, 10, 66, 520, 786, 1314)
