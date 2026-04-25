package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import com.voice.room.android.R
import com.voice.room.android.core.theme.MenaColors

/**
 * 我的 Tab 占位 Composable (T-30020 / 缺陷 #2 i18n)
 *
 * 居中显示占位文本，按系统 Locale 自动切换到 values-ar 阿语版本。
 * 后续 T-30024 替换为真正的个人中心页面。
 */
@Composable
fun ProfilePlaceholder() {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .testTag("profile_placeholder"),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = stringResource(id = R.string.profile_placeholder_label),
            style = MaterialTheme.typography.headlineMedium,
            color = MenaColors.OnBackgroundSecondary
        )
    }
}
