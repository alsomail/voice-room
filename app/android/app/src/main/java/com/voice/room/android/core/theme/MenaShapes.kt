package com.voice.room.android.core.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.unit.dp

/**
 * MenaShapes — 中东黑金主题形状定义
 *
 * - large:  16dp 圆角 — 大卡片
 * - medium: 24dp 圆角 — 按钮（胶囊型）
 * - small:  12dp 圆角 — 输入框
 */
val MenaShapes = Shapes(
    large = RoundedCornerShape(16.dp),
    medium = RoundedCornerShape(24.dp),
    small = RoundedCornerShape(12.dp),
)
