package com.voice.room.android.feature.room

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * HallTopBar — 大厅页顶部栏 (T-30022)
 *
 * - 金色标题 "VoiceRoom" (titleLarge, MenaColors.Primary)
 * - 搜索图标 (占位，点击无操作)
 * - testTag: hall_top_bar
 *
 * @param modifier 可选修饰符
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HallTopBar(modifier: Modifier = Modifier) {
    TopAppBar(
        modifier = modifier.testTag("hall_top_bar"),
        title = {
            Text(
                text = "VoiceRoom",
                style = MenaTypography.titleLarge,
                color = MenaColors.Primary,
            )
        },
        actions = {
            IconButton(onClick = { /* 占位：搜索功能待实现 */ }) {
                Icon(
                    imageVector = Icons.Default.Search,
                    contentDescription = "搜索",
                    tint = MenaColors.OnBackgroundSecondary,
                )
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = MenaColors.Background,
        ),
    )
}
