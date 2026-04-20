package com.voice.room.android.feature.room

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview

/**
 * 房间页顶部信息栏 (T-30009)
 *
 * 包含：
 * - 返回按钮（`testTag("room_back_button")`）
 * - 房间名（`testTag("room_name")`）
 * - 在线人数（`testTag("room_online_count")`）
 *
 * @param roomName    房间名称
 * @param onlineCount 在线人数
 * @param onBack      点击返回按钮的回调
 * @param modifier    可选 Modifier
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RoomTopBar(
    roomName: String,
    onlineCount: Int,
    onBack: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    TopAppBar(
        modifier = modifier.testTag("room_top_bar"),
        title = {
            Text(
                text = roomName,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.testTag("room_name"),
            )
        },
        navigationIcon = {
            IconButton(
                onClick = onBack,
                modifier = Modifier.testTag("room_back_button"),
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "返回",
                )
            }
        },
        actions = {
            Text(
                text = "$onlineCount",
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.testTag("room_online_count"),
            )
        },
    )
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "RoomTopBar — 预览")
@Composable
private fun RoomTopBarPreview() {
    RoomTopBar(
        roomName = "欢迎来到语聊房",
        onlineCount = 42,
    )
}
