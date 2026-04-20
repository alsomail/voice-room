package com.voice.room.android.feature.main

import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors

/**
 * MenaBottomNavigation — 底部导航栏封装组件 (T-30020)
 *
 * 三个 Tab：房间 / 消息 / 我的
 * - 选中色：MenaColors.Primary（金色 #D4AF37）
 * - 未选中色：MenaColors.OnBackgroundTertiary（灰色 #6C6C6C）
 * - 导航栏背景：MenaColors.Background（#1A1A2E）
 * - 指示器透明：indicatorColor = Color.Transparent
 *
 * @param currentRoute 当前选中路由（用于判断选中状态）
 * @param onTabSelected Tab 点击回调
 * @param modifier 外部传入的 Modifier
 */
@Composable
fun MenaBottomNavigation(
    currentRoute: String?,
    onTabSelected: (MainTab) -> Unit,
    modifier: Modifier = Modifier,
) {
    NavigationBar(
        modifier = modifier.testTag("bottom_nav"),
        containerColor = MenaColors.Background,
        tonalElevation = 0.dp,
    ) {
        MainTab.entries.forEach { tab ->
            val selected = currentRoute == tab.route
            NavigationBarItem(
                selected = selected,
                onClick = { onTabSelected(tab) },
                icon = {
                    Icon(
                        tab.icon,
                        contentDescription = tab.labelEn,
                        modifier = Modifier.size(24.dp)
                    )
                },
                label = {
                    Text(
                        tab.labelEn,
                        style = MaterialTheme.typography.labelSmall
                    )
                },
                colors = NavigationBarItemDefaults.colors(
                    selectedIconColor = MenaColors.Primary,
                    selectedTextColor = MenaColors.Primary,
                    unselectedIconColor = MenaColors.OnBackgroundTertiary,
                    unselectedTextColor = MenaColors.OnBackgroundTertiary,
                    indicatorColor = Color.Transparent,
                ),
                modifier = Modifier.testTag(tab.testTag),
            )
        }
    }
}
