package com.voice.room.android.feature.room.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * L2 麦位金色脉冲光圈 Modifier（T-30031）
 *
 * 当 [glowing] = true 时，为 Composable 添加金色 Scale 脉冲动画：
 * Scale **1.0 → 1.2 → 1.0**，循环 **2 次**，然后停止。
 *
 * 与原 alpha 脉冲不同，此版本严格按 TDS §L2 规格：
 * > Scale 1.0→1.2→1.0 循环 2 次，然后 `active=false`
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
    // Animatable 控制 scale，初始值 1.0
    val scale = remember { Animatable(1.0f) }

    // 每次 glowing 变为 true，执行 2 次 scale 脉冲循环（1.0 → 1.2 → 1.0）× 2
    LaunchedEffect(glowing) {
        if (glowing) {
            repeat(2) {
                scale.animateTo(
                    targetValue = 1.2f,
                    animationSpec = tween(durationMillis = 300, easing = FastOutSlowInEasing),
                )
                scale.animateTo(
                    targetValue = 1.0f,
                    animationSpec = tween(durationMillis = 300, easing = FastOutSlowInEasing),
                )
            }
        } else {
            scale.snapTo(1.0f)
        }
    }

    this
        .graphicsLayer(scaleX = scale.value, scaleY = scale.value)
        .border(width = width, color = color, shape = shape)
}
