package com.voice.room.android.core.theme

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingActionButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

/**
 * 金色渐变画笔 — 编译期常量色值，无需每次重组重建（MEDIUM-01 修复）
 */
internal val GoldGradientBrush = Brush.horizontalGradient(
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
            .padding(horizontal = 24.dp, vertical = 12.dp)
            // Round 3 BUG-002 修复：合并 Box + 内部 Text 的语义节点，
            // 这样 GoldButton 在 Compose 测试 merged-tree 中只暴露一个节点，
            // 既可被 testTag/onNodeWithText 唯一定位，也可参与 assertTextEquals。
            .semantics(mergeDescendants = true) {},
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            // 缺陷 #6 修复（WCAG AA）：金色渐变（#D4AF37→#FFD700）上的文字
            // 用深色 Background (#1A1A2E) 而非白色 OnBackground，可获得 ~7.5:1 对比度
            color = MenaColors.Background,
            style = MaterialTheme.typography.titleMedium,
        )
    }
}

/**
 * GoldFab — 金色渐变浮动操作按钮（TC-THEME-00002 修复）
 *
 * 使用 `containerColor = Color.Transparent` + `Modifier.background(GoldGradientBrush, CircleShape)`
 * 实现金色水平渐变，替代原来的纯色 `MenaColors.Primary`。
 *
 * @param onClick    点击回调
 * @param modifier   外部 Modifier（testTag 由调用方注入）
 * @param contentDescription 无障碍内容描述
 */
@Composable
fun GoldFab(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    contentDescription: String? = null,
) {
    FloatingActionButton(
        onClick = onClick,
        modifier = modifier.background(brush = GoldGradientBrush, shape = CircleShape),
        containerColor = Color.Transparent,
        contentColor = MenaColors.Background,
        elevation = FloatingActionButtonDefaults.elevation(),
    ) {
        Icon(
            imageVector = Icons.Default.Add,
            contentDescription = contentDescription,
        )
    }
}
