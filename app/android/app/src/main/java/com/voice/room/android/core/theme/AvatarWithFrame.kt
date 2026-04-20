package com.voice.room.android.core.theme

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage

/**
 * AvatarWithFrame — 圆形头像 + 可选金色光圈
 *
 * - 圆形头像（Coil AsyncImage + CircleShape clip）
 * - showFrame=true 时外圈金色描边 2dp（Primary 色 + CircleShape border）
 * - 默认占位图（Icons.Default.Person）
 *
 * @param imageUrl  头像 URL（null 时显示占位图）
 * @param size      图片区域尺寸（不含边框），默认 60.dp
 * @param showFrame 是否显示金色边框，默认 true
 * @param modifier  外部 Modifier
 */
@Composable
fun AvatarWithFrame(
    imageUrl: String?,
    modifier: Modifier = Modifier,
    size: Dp = 60.dp,
    showFrame: Boolean = true,
) {
    val borderWidth = 2.dp

    Box(
        modifier = modifier
            .then(
                if (showFrame) {
                    Modifier.size(size + borderWidth * 2)
                } else {
                    Modifier.size(size)
                }
            ),
        contentAlignment = Alignment.Center,
    ) {
        // 金色边框层（仅 showFrame=true 时渲染）
        if (showFrame) {
            Box(
                modifier = Modifier
                    .size(size + borderWidth * 2)
                    .testTag("avatar_frame")
                    .border(
                        width = borderWidth,
                        color = MenaColors.Primary,
                        shape = CircleShape,
                    )
            )
        }

        // 头像内容层
        if (imageUrl != null) {
            AsyncImage(
                model = imageUrl,
                contentDescription = "Avatar",
                modifier = Modifier
                    .size(size)
                    .clip(CircleShape),
                contentScale = ContentScale.Crop,
            )
        } else {
            // 占位图
            Icon(
                imageVector = Icons.Default.Person,
                contentDescription = "Default avatar",
                modifier = Modifier
                    .size(size)
                    .clip(CircleShape)
                    .testTag("avatar_placeholder"),
                tint = MenaColors.OnBackgroundSecondary,
            )
        }
    }
}
