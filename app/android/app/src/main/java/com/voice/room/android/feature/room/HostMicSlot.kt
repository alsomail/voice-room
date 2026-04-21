package com.voice.room.android.feature.room

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.AvatarWithFrame
import com.voice.room.android.core.theme.MenaColors

/**
 * 主麦槽位 Composable (T-30025)
 *
 * 设计规范：
 * - 有人时：AvatarWithFrame(80dp, showFrame=true) + Canvas 金色光圈脉冲动画 + 昵称
 * - 空位时：[EmptyHostSlotCircle]（虚线圆圈 + "+" 图标，80dp）
 *
 * 始终处于 slots[0] 位置（主麦），居中显示。
 *
 * testTag:
 * - 有人: `"mic_slot_occupied_0"`
 * - 空位: `"mic_slot_empty_0"`
 *
 * @param slot     麦位 UI 状态（index 应为 0）
 * @param modifier 可选 Modifier
 * @param onClick  点击回调，参数为 [MicSlotUi.index]
 */
@Composable
fun HostMicSlot(
    slot: MicSlotUi,
    modifier: Modifier = Modifier,
    onClick: (index: Int) -> Unit = {},
) {
    val isOccupied = slot.isOccupied

    val contentDesc = when {
        isOccupied -> "主麦位，${slot.nickname}，点击互动"
        else       -> "主麦位，空位，点击上麦"
    }

    Column(
        modifier = modifier
            .testTag(if (isOccupied) "mic_slot_occupied_${slot.index}" else "mic_slot_empty_${slot.index}")
            .clickable(onClickLabel = contentDesc) { onClick(slot.index) }
            .semantics { contentDescription = contentDesc },
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier.padding(vertical = 12.dp),
            contentAlignment = Alignment.Center,
        ) {
            if (isOccupied) {
                // ── 有人：Canvas 金色光圈 + AvatarWithFrame(80dp) ──────────────
                GoldGlowRing(size = AVATAR_SIZE_HOST_DP)
                AvatarWithFrame(
                    imageUrl = slot.avatarUrl,
                    size = AVATAR_SIZE_HOST_DP,
                    showFrame = true,
                )
            } else {
                // ── 空位：虚线圆圈 + "+" 图标（80dp）────────────────────────────
                EmptyHostSlotCircle(size = AVATAR_SIZE_HOST_DP)
            }
        }

        // 昵称（有人时显示）
        if (isOccupied && slot.nickname != null) {
            Text(
                text = slot.nickname,
                style = MaterialTheme.typography.bodyMedium,
                color = MenaColors.Primary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("host_mic_nickname"),
            )
        }
    }
}

// ─── 私有子组件 ────────────────────────────────────────────────────────────────

/** 主麦 avatar 尺寸常量 (80dp) */
private val AVATAR_SIZE_HOST_DP = 80.dp

/**
 * 金色光圈脉冲动画（仅主麦有人时显示）。
 *
 * Canvas 绘制：
 * - drawCircle Stroke 6dp 宽，金色 alpha=0.35
 * - infiniteRepeatable 脉冲（scale 0.92→1.08，1200ms，RepeatMode.Reverse）
 */
@Composable
private fun GoldGlowRing(size: androidx.compose.ui.unit.Dp) {
    // 光圈外扩：stroke 半径 = avatarRadius(40dp) + 12dp = 52dp
    // stroke 宽 6dp → 外缘 = 52dp + 3dp(半宽) = 55dp
    // Canvas 边界需 ≥ 55dp * 2 = 110dp → glowSize = size(80dp) + 30dp
    val glowSize = size + 30.dp

    val infiniteTransition = rememberInfiniteTransition(label = "host_glow")
    val scale by infiniteTransition.animateFloat(
        initialValue = 0.92f,
        targetValue = 1.08f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1200),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "glow_scale",
    )

    Canvas(
        modifier = Modifier
            .size(glowSize)
            .graphicsLayer(scaleX = scale, scaleY = scale),
    ) {
        val strokeWidthPx = 6.dp.toPx()
        val radius = (size.toPx() / 2) + 12.dp.toPx()
        drawCircle(
            color = MenaColors.Primary.copy(alpha = 0.35f),
            radius = radius,
            style = Stroke(width = strokeWidthPx),
        )
    }
}

/**
 * 主麦空位：虚线圆圈（80dp）+ 金色 "+" 图标
 *
 * Canvas 绘制虚线圆形边框，center 叠加 Icon(Add)。
 */
@Composable
private fun EmptyHostSlotCircle(size: androidx.compose.ui.unit.Dp) {
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
            modifier = Modifier.size(32.dp),
            tint = MenaColors.Primary,
        )
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, backgroundColor = 0xFF1A1A2E, name = "HostMicSlot — 有人")
@Composable
private fun HostMicSlotOccupiedPreview() {
    HostMicSlot(
        slot = MicSlotUi(index = 0, userId = "host-1", nickname = "Alice", avatarUrl = null, isMuted = false)
    )
}

@Preview(showBackground = true, backgroundColor = 0xFF1A1A2E, name = "HostMicSlot — 空位")
@Composable
private fun HostMicSlotEmptyPreview() {
    HostMicSlot(
        slot = MicSlotUi(index = 0, userId = null, nickname = null, avatarUrl = null, isMuted = false)
    )
}
