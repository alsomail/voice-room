package com.voice.room.android.feature.room

import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * 分类 Tab 常量列表 — 被 [CategoryTabRow] 和 JVM 单测共用
 */
val CATEGORY_TABS: List<String> = listOf("热门", "新开", "关注", "游戏")

/**
 * 默认选中 Tab 索引
 */
const val DEFAULT_SELECTED_TAB_INDEX: Int = 0

/**
 * Phase 0.5 中，仅 index=0 的 "热门" tab 可交互，其余禁用
 */
fun isCategoryTabEnabled(index: Int): Boolean = index == 0

/**
 * CategoryTabRow — 分类横滑 (T-30022)
 *
 * - ScrollableTabRow + tabs ["热门","新开","关注","游戏"]
 * - 选中 tab: MenaColors.Primary 文字 + 下划线
 * - 未选中: MenaColors.OnBackgroundTertiary 文字
 * - Phase 0.5 仅 index=0 "热门" 可交互，其余 enabled=false
 * - "热门" tab testTag: category_tab_hot
 *
 * @param selectedIndex 当前选中索引
 * @param onTabSelected tab 切换回调
 * @param modifier 可选修饰符
 */
@Composable
fun CategoryTabRow(
    selectedIndex: Int = DEFAULT_SELECTED_TAB_INDEX,
    onTabSelected: (Int) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    ScrollableTabRow(
        selectedTabIndex = selectedIndex,
        modifier = modifier,
        containerColor = MenaColors.Background,
        contentColor = MenaColors.Primary,
        edgePadding = 16.dp,
    ) {
        CATEGORY_TABS.forEachIndexed { index, label ->
            val isSelected = index == selectedIndex
            val enabled = isCategoryTabEnabled(index)

            Tab(
                selected = isSelected,
                onClick = { if (enabled) onTabSelected(index) },
                enabled = enabled,
                modifier = if (index == 0) Modifier.testTag("category_tab_hot") else Modifier,
            ) {
                Text(
                    text = label,
                    style = MenaTypography.bodyMedium,
                    color = when {
                        isSelected -> MenaColors.Primary
                        else -> MenaColors.OnBackgroundTertiary
                    },
                )
            }
        }
    }
}
