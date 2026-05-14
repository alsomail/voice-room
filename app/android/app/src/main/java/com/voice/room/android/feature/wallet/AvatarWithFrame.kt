package com.voice.room.android.feature.wallet

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.voice.room.android.feature.noble.NOBLE_COLORS

/**
 * AvatarWithFrame — 带贵族框的头像组件 (T-30074)
 *
 * 基于 nobleTierLevel 渲染对应颜色的外框；
 * tierLevel=null 时仅显示头像，无外框。
 * frameUrl 由 Server 下发，使用 Coil 缓存。
 */
@Composable
fun AvatarWithFrame(
    avatarUrl: String?,
    nobleTierLevel: Int?,
    frameUrl: String?,
    userId: String,
    modifier: Modifier = Modifier,
    size: Int = 48
) {
    val frameColor = if (nobleTierLevel != null && nobleTierLevel in 1..6) {
        NOBLE_COLORS[nobleTierLevel - 1]
    } else null

    val frameWidth = if (frameColor != null) 3.dp else 0.dp

    Box(
        modifier = modifier
            .size(size.dp)
            .then(
                if (frameColor != null) Modifier
                    .border(frameWidth, frameColor, CircleShape)
                    .padding(frameWidth)
                else Modifier
            )
            .clip(CircleShape)
            .background(Color.DarkGray)
            .testTag("avatar_frame_$userId"),
        contentAlignment = Alignment.Center
    ) {
        if (frameUrl != null) {
            AsyncImage(
                model = frameUrl,
                contentDescription = "Frame",
                modifier = Modifier
                    .size((size + 8).dp)
                    .clip(CircleShape),
                contentScale = ContentScale.Crop
            )
        }

        AsyncImage(
            model = avatarUrl,
            contentDescription = "Avatar",
            modifier = Modifier
                .size(size.dp)
                .clip(CircleShape)
                .background(Color.DarkGray),
            contentScale = ContentScale.Crop
        )
    }
}
