package com.voice.room.android.feature.room.announcement

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp

/**
 * 公告弹窗组件（T-30043 AN43-01/AN43-04/AN43-08）
 *
 * 弹出展示房间公告，支持长文本滚动，提供关闭按钮。
 *
 * testTag:
 * - 弹窗根容器：`announcement_popup`
 * - 关闭按钮：`btn_announcement_close`
 *
 * @param announcement 公告文本（非空）
 * @param onDismiss    关闭弹窗回调
 */
@Composable
fun AnnouncementPopup(
    announcement: String,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    AlertDialog(
        modifier = modifier.testTag("announcement_popup"),
        onDismissRequest = onDismiss,
        title = {
            Text(
                text = "📢 房间公告",
                style = MaterialTheme.typography.titleMedium,
            )
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
            ) {
                Text(
                    text = announcement,
                    style = MaterialTheme.typography.bodyMedium,
                )
                Spacer(modifier = Modifier.height(8.dp))
            }
        },
        confirmButton = {
            Button(
                modifier = Modifier
                    .padding(bottom = 8.dp, end = 8.dp)
                    .testTag("btn_announcement_close"),
                onClick = onDismiss,
            ) {
                Text("知道了")
            }
        }
    )
}
