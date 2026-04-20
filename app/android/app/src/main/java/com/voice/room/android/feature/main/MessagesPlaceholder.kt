package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.voice.room.android.core.theme.MenaColors

/**
 * 消息 Tab 占位 Composable (T-30020)
 *
 * 居中显示 "Messages" 占位文本，后续 T-30023 替换为真正的消息列表。
 */
@Composable
fun MessagesPlaceholder() {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .testTag("messages_placeholder"),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = "Messages",
            style = MaterialTheme.typography.headlineMedium,
            color = MenaColors.OnBackgroundSecondary
        )
    }
}
