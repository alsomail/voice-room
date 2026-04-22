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
import androidx.compose.ui.graphics.painter.ColorPainter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.voice.room.android.feature.room.effect.GiftMessageUi

/**
 * L1 礼物弹幕气泡（T-30031）
 *
 * 以半透明金色背景显示礼物赠送信息：
 * "[sender] 送给 [receiver] [giftName] x[count]"
 *
 * effectLevel >= 3 时文字粗体展示（[GiftMessageUi.isBold]）。
 *
 * - 头像 16dp：使用 Coil AsyncImage 加载 [GiftMessageUi.senderAvatar]，失败时显示半透明白色占位
 * - 礼物图标 24dp：使用 Coil AsyncImage 加载 [GiftMessageUi.giftIconUrl]
 * - testTag: `gift_msg_{msgId}` 供 UI 自动化测试定位
 */
@Composable
fun GiftChatMessage(
    msg: GiftMessageUi,
    modifier: Modifier = Modifier,
) {
    // TDS testTag: gift_msg_{msgId}（HIGH-2）
    Box(
        modifier = modifier
            .testTag("gift_msg_${msg.msgId}")
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 2.dp)
            .background(
                color = Color(0xCC_FF_AA_00), // 半透明金色
                shape = RoundedCornerShape(16.dp),
            )
            .padding(horizontal = 12.dp, vertical = 6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            // 头像 16dp（TDS spec），Coil AsyncImage 加载，失败时显示半透明白色占位（MEDIUM-3）
            val avatarPlaceholder = ColorPainter(Color(0x80_FF_FF_FF))
            AsyncImage(
                model = msg.senderAvatar,
                contentDescription = "sender avatar",
                modifier = Modifier
                    .size(16.dp)
                    .clip(CircleShape),
                contentScale = ContentScale.Crop,
                placeholder = avatarPlaceholder,
                error = avatarPlaceholder,
                fallback = avatarPlaceholder,
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
            // 礼物图标 24dp（TDS spec），MEDIUM-3
            if (msg.giftIconUrl.isNotEmpty()) {
                Spacer(modifier = Modifier.width(4.dp))
                AsyncImage(
                    model = msg.giftIconUrl,
                    contentDescription = "gift icon",
                    modifier = Modifier.size(24.dp),
                    contentScale = ContentScale.Fit,
                    placeholder = ColorPainter(Color(0x40_FF_D7_00)),
                    error = ColorPainter(Color(0x40_FF_D7_00)),
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
 * ### testTag（HIGH-2）
 * - 外层容器：`fullscreen_gift_overlay`
 * - 跳过点击区：`btn_skip_fullscreen_gift`
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
    // TDS testTag: fullscreen_gift_overlay（HIGH-2）
    Box(
        modifier = modifier
            .testTag("fullscreen_gift_overlay")
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
        // 点击任意区域跳过；TDS testTag: btn_skip_fullscreen_gift（HIGH-2）
        Box(
            modifier = Modifier
                .testTag("btn_skip_fullscreen_gift")
                .fillMaxSize()
                .clickable(onClick = onSkip),
        )
    }
}
