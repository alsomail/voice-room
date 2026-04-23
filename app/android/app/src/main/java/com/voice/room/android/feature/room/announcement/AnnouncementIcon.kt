package com.voice.room.android.feature.room.announcement

import androidx.compose.foundation.clickable
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Article
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag

/**
 * 顶部公告图标组件 📄（T-30043 AN43-03/AN43-04）
 *
 * 仅在房间有非空公告时显示。点击后展示公告弹窗（[AnnouncementPopup]）。
 *
 * testTag: `btn_show_announcement`
 *
 * @param onClick 点击回调（触发弹窗展示）
 */
@Composable
fun AnnouncementIcon(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Icon(
        imageVector = Icons.Filled.Article,
        contentDescription = "查看公告",
        tint = LocalContentColor.current,
        modifier = modifier
            .testTag("btn_show_announcement")
            .clickable(onClick = onClick),
    )
}
