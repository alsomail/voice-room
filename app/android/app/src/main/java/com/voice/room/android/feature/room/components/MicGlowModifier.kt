package com.voice.room.android.feature.room.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * L2 麦位金色脉冲光圈 Modifier（T-30031）
 *
 * 当 [glowing] = true 时，为 Composable 添加金色脉冲边框动画（0.5x ~ 1x 透明度交替）。
 *
 * 使用示例：
 * ```
 * MicSlotCard(
 *     modifier = Modifier.micGlow(
 *         glowing = micGlowTargetUserId == slot.userId,
 *         shape = CircleShape,
 *     )
 * )
 * ```
 *
 * @param glowing  是否播放光圈动画
 * @param shape    边框形状（需与宿主 Composable 的 clip shape 匹配）
 * @param width    边框宽度（默认 2dp）
 * @param color    光圈颜色（默认金色）
 */
fun Modifier.micGlow(
    glowing: Boolean,
    shape: Shape,
    width: Dp = 2.dp,
    color: Color = Color(0xFF_FF_D7_00), // 金色
): Modifier = if (!glowing) this else composed {
    val infiniteTransition = rememberInfiniteTransition(label = "micGlow")
    val alpha by infiniteTransition.animateFloat(
        initialValue = 0.5f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "micGlowAlpha",
    )
    this.border(
        width = width,
        color = color.copy(alpha = alpha),
        shape = shape,
    )
}
