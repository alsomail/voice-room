package com.voice.room.android.core.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors

/**
 * PlaceholderScreen 设计规范常量 (T-30023)
 *
 * 提取为独立对象以支持 JVM 单测验证，避免无意修改设计规范。
 */
object PlaceholderScreenDefaults {
    /** 图标尺寸 (dp) */
    const val ICON_SIZE_DP = 64
    /** 图标到标题的间距 (dp) */
    const val ICON_TO_TITLE_SPACING_DP = 8
    /** 标题到副标题的间距 (dp) */
    const val TITLE_TO_SUBTITLE_SPACING_DP = 4
}

/**
 * PlaceholderScreen — 通用"功能即将上线"占位页 Composable (T-30023)
 *
 * 可复用组件，接受 icon / title / subtitle 参数，用于未开发功能的占位展示。
 * 背景由外层 Scaffold 的 containerColor (MenaColors.Background) 提供，
 * 组件本身不设背景以避免重复绘制。
 *
 * 视觉规范:
 * - 图标: 64dp, 颜色 MenaColors.OnBackgroundTertiary (#6C6C6C), 可选
 * - 标题: titleMedium (16sp Bold), 颜色 MenaColors.OnBackgroundSecondary (#B0B0B0)
 * - 副标题: bodySmall (12sp), 颜色 MenaColors.OnBackgroundTertiary (#6C6C6C), 可选
 * - 布局: 垂直居中, 图标→8dp→标题→4dp→副标题
 *
 * @param title 主标题文字（必传）
 * @param modifier 外部可传入的 Modifier
 * @param icon 可选图标 ImageVector
 * @param subtitle 可选副标题文字
 */
@Composable
fun PlaceholderScreen(
    title: String,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    subtitle: String? = null,
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .testTag("placeholder_screen"),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        if (icon != null) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier
                    .size(PlaceholderScreenDefaults.ICON_SIZE_DP.dp)
                    .testTag("placeholder_icon"),
                tint = MenaColors.OnBackgroundTertiary,
            )
            Spacer(modifier = Modifier.height(PlaceholderScreenDefaults.ICON_TO_TITLE_SPACING_DP.dp))
        }
        Text(
            text = title,
            modifier = Modifier.testTag("placeholder_title"),
            style = MaterialTheme.typography.titleMedium,
            color = MenaColors.OnBackgroundSecondary,
        )
        if (subtitle != null) {
            Spacer(modifier = Modifier.height(PlaceholderScreenDefaults.TITLE_TO_SUBTITLE_SPACING_DP.dp))
            Text(
                text = subtitle,
                modifier = Modifier.testTag("placeholder_subtitle"),
                style = MaterialTheme.typography.bodySmall,
                color = MenaColors.OnBackgroundTertiary,
            )
        }
    }
}
