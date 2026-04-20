package com.voice.room.android.core.theme

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp

/**
 * 金色渐变画笔 — 编译期常量色值，无需每次重组重建（MEDIUM-01 修复）
 */
private val GoldGradientBrush = Brush.horizontalGradient(
    colors = listOf(MenaColors.Primary, MenaColors.PrimaryBright)
)

/**
 * GoldButton — 金色渐变胶囊按钮
 *
 * - 金色水平渐变背景 (Primary → PrimaryBright)
 * - 白色文字 (OnBackground)
 * - 24dp 圆角（胶囊 capsule shape）
 * - enabled=false 时透明度降至 38%，不可点击
 *
 * @param text     按钮文字
 * @param onClick  点击回调
 * @param modifier 外部 Modifier（testTag 由调用方注入）
 * @param enabled  是否可点击，默认 true
 */
@Composable
fun GoldButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val shape = RoundedCornerShape(24.dp)

    Box(
        modifier = modifier
            .alpha(if (enabled) 1f else 0.38f)
            .clip(shape)
            .background(brush = GoldGradientBrush, shape = shape)
            .clickable(
                enabled = enabled,
                onClick = onClick,
                role = Role.Button,
            )
            .padding(horizontal = 24.dp, vertical = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            color = MenaColors.OnBackground,
            style = MaterialTheme.typography.titleMedium,
        )
    }
}
