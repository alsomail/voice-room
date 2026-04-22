package com.voice.room.android.feature.room.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.feature.room.effect.GiftMessageUi

/**
 * L1 礼物弹幕气泡（T-30031）
 *
 * 以半透明金色背景显示礼物赠送信息：
 * "[sender] 送给 [receiver] [giftName] x[count]"
 *
 * effectLevel >= 3 时文字粗体展示（[GiftMessageUi.isBold]）。
 *
 * 注意：防腐层 — 不依赖任何 Lottie SDK，仅展示礼物图标 URL（生产时配合 Coil 加载）。
 */
@Composable
fun GiftChatMessage(
    msg: GiftMessageUi,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 2.dp)
            .background(
                color = Color(0xCC_FF_AA_00), // 半透明金色
                shape = RoundedCornerShape(16.dp),
            )
            .padding(horizontal = 12.dp, vertical = 6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            // 头像占位（生产时用 Coil AsyncImage 替换）
            Box(
                modifier = Modifier
                    .size(24.dp)
                    .clip(CircleShape)
                    .background(Color(0x80_FF_FF_FF)),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Column {
                val fontWeight = if (msg.isBold) FontWeight.Bold else FontWeight.Normal
                Text(
                    text = buildAnnotatedString {
                        withStyle(SpanStyle(color = Color.White, fontWeight = fontWeight)) {
                            append(msg.senderNickname)
                            append(" 送给 ")
                            append(msg.receiverNickname)
                            append(" ")
                        }
                        withStyle(SpanStyle(color = Color(0xFF_FF_D7_00), fontWeight = fontWeight)) {
                            append(msg.giftName)
                            if (msg.count > 1) {
                                append(" ×${msg.count}")
                            }
                        }
                    },
                    fontSize = 12.sp,
                )
            }
        }
    }
}

/**
 * L3 全屏礼物特效遮罩（T-30031）
 *
 * 占满整个屏幕，展示动画 URL 对应的 Lottie 动画（或本地 fallback 动画）。
 * 点击可跳过（调用 [onSkip]）。
 *
 * ### 防腐层说明
 * - 不直接 import 任何 `com.airbnb.lottie.*`
 * - MVP 用纯 Compose Box 做 fallback 演示；接入 Lottie 时
 *   仅替换此 Composable 的内部实现，外部调用点不变
 *
 * @param animationUrl Lottie JSON URL（空字符串时展示 fallback 动画）
 * @param onSkip       用户点击屏幕跳过动画
 */
@Composable
fun GiftFullscreenOverlay(
    animationUrl: String,
    onSkip: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color(0x99_00_00_00)), // 半透明黑色遮罩
        contentAlignment = Alignment.Center,
    ) {
        // MVP fallback：文字展示 URL（生产接入 Lottie 时替换此处）
        Text(
            text = if (animationUrl.isNotEmpty()) "🎉 全屏特效：$animationUrl" else "🎉 全屏特效（本地动画）",
            color = Color.White,
            fontSize = 16.sp,
            modifier = Modifier.padding(24.dp),
        )
        // 点击任意区域跳过
        Box(
            modifier = Modifier
                .fillMaxSize()
                .clickable(onClick = onSkip),
        )
    }
}
