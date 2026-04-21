package com.voice.room.android.feature.room

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.AvatarWithFrame
import com.voice.room.android.core.theme.MenaColors

/**
 * 麦位卡片 — T-30011
 *
 * 三态：
 * - EMPTY  (`userId == null`)：「+」图标 + 1-based 座位序号
 * - OCCUPIED (`isOccupied && !isMuted`)：头像 + 昵称 + 音浪动画占位
 * - MUTED   (`isOccupied && isMuted`)：头像 + 昵称 + 禁麦图标
 *
 * @param slot     麦位 UI 状态
 * @param modifier 可选 Modifier
 * @param onClick  点击回调，参数为 [MicSlotUi.index]
 */
@Composable
fun MicSlotCard(
    slot: MicSlotUi,
    modifier: Modifier = Modifier,
    onClick: (index: Int) -> Unit = {},
) {
    val isOccupied = slot.isOccupied
    val isMuted = isOccupied && slot.isMuted

    // 无障碍 contentDescription（MC-08/09/10）
    val contentDesc = when {
        !isOccupied -> "麦位 ${slot.index + 1}，空位，点击上麦"
        isMuted     -> "麦位 ${slot.index + 1}，${slot.nickname}，已禁麦"
        else        -> "麦位 ${slot.index + 1}，${slot.nickname}，点击互动"
    }

    Box(
        modifier = modifier
            .testTag(if (!isOccupied) "mic_slot_empty_${slot.index}" else "mic_slot_occupied_${slot.index}")
            .clickable(onClickLabel = contentDesc) { onClick(slot.index) }
            .semantics { contentDescription = contentDesc },  // 内层 = 被 clickable mergeDescendants 合并
        contentAlignment = Alignment.Center,
    ) {
        if (!isOccupied) {
            // ── EMPTY ── 黑金风格：虚线圆圈 + 金色"+"图标 + 座位序号 (T-30025) ──
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                EmptySlotCircle(size = AVATAR_SIZE_GUEST_DP)
                Text(
                    text = "${slot.index + 1}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MenaColors.Primary,
                )
            }
        } else {
            // ── OCCUPIED / MUTED ── 黑金风格：AvatarWithFrame(60dp) (T-30025) ──
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Box {
                    // 60dp AvatarWithFrame：有人显示金色边框，静音不显示边框
                    AvatarWithFrame(
                        imageUrl = slot.avatarUrl,
                        size = AVATAR_SIZE_GUEST_DP,
                        showFrame = !isMuted,
                    )

                    // 音浪动画占位：仅 OCCUPIED（非静音）时显示（MC-02 / MC-03）
                    // HIGH-02: 使用 AnimatedVisibility 替代裸 if，避免切换时视觉突变。
                    // 通过私有 wrapper 断开外层 ColumnScope 的隐式 receiver 传播，
                    // 防止编译器错误地选择 ColumnScope.AnimatedVisibility 扩展。
                    AnimatedSoundWave(
                        visible = !isMuted,
                        waveModifier = Modifier
                            .matchParentSize()
                            .testTag("mic_slot_sound_wave"),
                    )

                    // 禁麦图标：仅 MUTED 时显示（MC-03），RTL 安全使用 BottomEnd（§2.5）
                    if (isMuted) {
                        Icon(
                            imageVector = Icons.Default.MicOff,
                            contentDescription = null,
                            modifier = Modifier
                                .align(Alignment.BottomEnd)
                                .size(16.dp)
                                .testTag("mic_slot_muted_icon_${slot.index}"),
                            tint = Color.Red,
                        )
                    }
                }
                Text(
                    text = slot.nickname ?: "",
                    style = MaterialTheme.typography.labelSmall,
                    color = MenaColors.OnBackground,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

/** 副麦 avatar 尺寸常量 (60dp) */
private val AVATAR_SIZE_GUEST_DP = 60.dp

/**
 * 空麦位通用子组件（T-30025）
 *
 * 虚线圆形边框 + 金色 "+" 图标，尺寸由 [size] 参数控制。
 * 使用 Canvas drawCircle + PathEffect.dashPathEffect(floatArrayOf(8f, 6f))。
 *
 * @param size  整体尺寸（主麦 80dp，副麦 60dp）
 */
@Composable
private fun EmptySlotCircle(size: androidx.compose.ui.unit.Dp) {
    Box(
        modifier = Modifier.size(size),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(modifier = Modifier.size(size)) {
            val strokeWidthPx = 2.dp.toPx()
            val radius = (this.size.minDimension - strokeWidthPx) / 2f
            drawCircle(
                color = MenaColors.Primary.copy(alpha = 0.6f),
                radius = radius,
                style = Stroke(
                    width = strokeWidthPx,
                    pathEffect = PathEffect.dashPathEffect(floatArrayOf(8f, 6f)),
                ),
            )
        }
        Icon(
            imageVector = Icons.Default.Add,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MenaColors.Primary,
        )
    }
}

/**
 * AnimatedVisibility 的私有 wrapper。
 *
 * 将 [AnimatedVisibility] 移至独立 @Composable，避免在
 * `Column { Box { AnimatedVisibility } }` 嵌套结构中，Kotlin 因外层 ColumnScope
 * 的隐式 receiver 优先选择 `ColumnScope.AnimatedVisibility` 扩展而编译报错。
 *
 * [waveModifier] 在调用处（BoxScope 内）求值，支持 `matchParentSize()`。
 */
@Composable
private fun AnimatedSoundWave(visible: Boolean, waveModifier: Modifier = Modifier) {
    AnimatedVisibility(visible = visible) {
        SoundWaveAnimationPlaceholder(modifier = waveModifier)
    }
}

/**
 * 音浪动画占位：绿色脉冲圆圈（MVP 阶段无需 Lottie — §2.4）
 */
@Composable
private fun SoundWaveAnimationPlaceholder(modifier: Modifier = Modifier) {
    val infiniteTransition = rememberInfiniteTransition(label = "soundwave")
    val scale by infiniteTransition.animateFloat(
        initialValue = 0.85f,
        targetValue = 1.15f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "scale",
    )
    Box(
        modifier = modifier
            .graphicsLayer(scaleX = scale, scaleY = scale)
            .background(
                color = Color(0xFF4CAF50).copy(alpha = 0.25f),
                shape = CircleShape,
            ),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────────────────────────────────────

@Preview(showBackground = true, name = "MicSlotCard — EMPTY")
@Composable
private fun PreviewEmpty() =
    MicSlotCard(slot = MicSlotUi(index = 2, userId = null, nickname = null, avatarUrl = null, isMuted = false))

@Preview(showBackground = true, name = "MicSlotCard — OCCUPIED")
@Composable
private fun PreviewOccupied() =
    MicSlotCard(slot = MicSlotUi(index = 0, userId = "u1", nickname = "Alice", avatarUrl = null, isMuted = false))

@Preview(showBackground = true, name = "MicSlotCard — MUTED")
@Composable
private fun PreviewMuted() =
    MicSlotCard(slot = MicSlotUi(index = 1, userId = "u2", nickname = "Bob", avatarUrl = null, isMuted = true))
