package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.Box
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Chat
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.voice.room.android.core.ui.PlaceholderScreen

/**
 * 消息 Tab 占位 Composable (T-30020 创建, T-30023 升级)
 *
 * 委托给通用 PlaceholderScreen 组件，传入消息 Tab 特定参数。
 * 外层 Box 保留 testTag("messages_placeholder") 确保 T-30020 测试不回归。
 *
 * @see PlaceholderScreen
 */
@Composable
fun MessagesPlaceholder() {
    Box(modifier = Modifier.testTag("messages_placeholder")) {
        PlaceholderScreen(
            icon = Icons.AutoMirrored.Outlined.Chat,
            title = "消息功能即将上线",
            subtitle = "敬请期待",
        )
    }
}
