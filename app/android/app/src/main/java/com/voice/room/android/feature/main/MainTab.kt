package com.voice.room.android.feature.main

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Chat
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Person
import androidx.compose.ui.graphics.vector.ImageVector

/**
 * MainTab — 底部三 Tab 枚举定义 (T-30020)
 *
 * 每个 Tab 包含导航路由、图标、中英阿标签和测试标识。
 *
 * @property route   NavHost 内部导航路由（如 "main/rooms"）
 * @property icon    底部导航栏图标
 * @property labelEn 英文标签
 * @property labelAr 阿拉伯语标签
 * @property testTag Compose UI 测试标识
 */
enum class MainTab(
    val route: String,
    val icon: ImageVector,
    val labelEn: String,
    val labelAr: String,
    val testTag: String,
) {
    ROOMS("main/rooms", Icons.Default.Home, "Rooms", "الغرف", "tab_rooms"),
    MESSAGES("main/messages", Icons.AutoMirrored.Filled.Chat, "Messages", "الرسائل", "tab_messages"),
    PROFILE("main/profile", Icons.Default.Person, "Me", "حسابي", "tab_profile"),
}
