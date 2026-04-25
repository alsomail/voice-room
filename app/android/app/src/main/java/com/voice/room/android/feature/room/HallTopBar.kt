package com.voice.room.android.feature.room

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.EmojiEvents
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
import androidx.compose.ui.res.stringResource
import com.voice.room.android.R
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * HallTopBar — 大厅页顶部栏 (T-30022, T-30033 升级)
 *
 * - 金色标题 "VoiceRoom" (titleLarge, MenaColors.Primary)
 * - 搜索图标 (占位，点击无操作)
 * - 🏆 榜单图标 (点击跳转排行榜页)
 * - testTag: hall_top_bar
 *
 * @param modifier          可选修饰符
 * @param onNavigateToRanking 点击🏆时回调（默认无操作）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HallTopBar(
    modifier: Modifier = Modifier,
    onNavigateToRanking: () -> Unit = {},
) {
    TopAppBar(
        modifier = modifier.testTag("hall_top_bar"),
        title = {
            Text(
                text = stringResource(id = R.string.hall_top_bar_title),
                style = MenaTypography.titleLarge,
                color = MenaColors.Primary,
            )
        },
        actions = {
            // 榜单入口 (T-30033)
            IconButton(
                onClick = onNavigateToRanking,
                modifier = Modifier.testTag("hall_ranking_button"),
            ) {
                Icon(
                    imageVector = Icons.Filled.EmojiEvents,
                    contentDescription = stringResource(id = R.string.hall_ranking_action),
                    tint = MenaColors.Primary,
                )
            }
            IconButton(onClick = { /* 占位：搜索功能待实现 */ }) {
                Icon(
                    imageVector = Icons.Default.Search,
                    contentDescription = stringResource(id = R.string.hall_search_action),
                    tint = MenaColors.OnBackgroundSecondary,
                )
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = MenaColors.Background,
        ),
    )
}
