package com.voice.room.android.feature.main

import androidx.annotation.StringRes
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Chat
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Person
import androidx.compose.ui.graphics.vector.ImageVector
import com.voice.room.android.R

/**
 * MainTab — 底部三 Tab 枚举定义 (T-30020 / 缺陷 #3 修复)
 *
 * 缺陷 #3：原先的 `labelEn` / `labelAr` 双字段是死代码（仅 `labelEn` 被读取，
 * 阿语 Locale 用户也只看到英文）。现统一为 [labelRes]（@StringRes），
 * 由 Composable 通过 `stringResource(tab.labelRes)` 解析，让系统按 values-ar
 * 自动切换到阿拉伯文。
 *
 * @property route     NavHost 内部导航路由（如 "main/rooms"）
 * @property icon      底部导航栏图标
 * @property labelRes  Tab 标签字符串资源 ID（values / values-ar）
 * @property testTag   Compose UI 测试标识
 */
enum class MainTab(
    val route: String,
    val icon: ImageVector,
    @StringRes val labelRes: Int,
    val testTag: String,
) {
    ROOMS("main/rooms", Icons.Default.Home, R.string.tab_rooms, "tab_rooms"),
    MESSAGES("main/messages", Icons.AutoMirrored.Filled.Chat, R.string.tab_messages, "tab_messages"),
    PROFILE("main/profile", Icons.Default.Person, R.string.tab_profile, "tab_profile"),
}
