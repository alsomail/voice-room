package com.voice.room.android.core.theme

import androidx.compose.ui.graphics.Color

/**
 * MenaColors — 中东黑金主题色彩令牌
 *
 * 11 个颜色常量 + 对应 ULong 原始值（用于 JVM 单测不依赖 Compose）
 *
 * 色值来源: doc/design/android/T-30018.md
 */
object MenaColors {

    // ── 原始值常量（JVM 单测可用） ────────────────────
    const val BACKGROUND_VALUE: ULong              = 0xFF1A1A2EuL
    const val SURFACE_VALUE: ULong                 = 0xFF16213EuL
    const val SURFACE_VARIANT_VALUE: ULong         = 0xFF0F3460uL
    const val PRIMARY_VALUE: ULong                 = 0xFFD4AF37uL
    const val PRIMARY_BRIGHT_VALUE: ULong          = 0xFFFFD700uL
    const val ON_BACKGROUND_VALUE: ULong           = 0xFFFFFFFFuL
    const val ON_BACKGROUND_SECONDARY_VALUE: ULong = 0xFFB0B0B0uL
    const val ON_BACKGROUND_TERTIARY_VALUE: ULong  = 0xFF6C6C6CuL
    const val ERROR_VALUE: ULong                   = 0xFFE74C3CuL
    const val SUCCESS_VALUE: ULong                 = 0xFF2ECC71uL
    const val SYSTEM_MESSAGE_VALUE: ULong          = 0xFFF39C12uL
    const val CHAT_BUBBLE_VALUE: ULong             = 0xFF2A2A2AuL

    // ── Compose Color 常量 ────────────────────────
    // 注意：使用 .toInt() 强制走 Color(color: Int) 重载，按 sRGB ARGB 解码。
    // 直接传 ULong 会调到 Color(value: ULong)，把低 6 位当作 colorspace ID，
    // 在 Android 13 上触发 ArrayIndexOutOfBoundsException(length=18) — BUG-ANDROID-001。
    val Background: Color            = Color(BACKGROUND_VALUE.toInt())
    val Surface: Color               = Color(SURFACE_VALUE.toInt())
    val SurfaceVariant: Color        = Color(SURFACE_VARIANT_VALUE.toInt())
    val Primary: Color               = Color(PRIMARY_VALUE.toInt())
    val PrimaryBright: Color         = Color(PRIMARY_BRIGHT_VALUE.toInt())
    val OnBackground: Color          = Color(ON_BACKGROUND_VALUE.toInt())
    val OnBackgroundSecondary: Color = Color(ON_BACKGROUND_SECONDARY_VALUE.toInt())
    val OnBackgroundTertiary: Color  = Color(ON_BACKGROUND_TERTIARY_VALUE.toInt())
    val Error: Color                 = Color(ERROR_VALUE.toInt())
    val Success: Color               = Color(SUCCESS_VALUE.toInt())
    val SystemMessage: Color         = Color(SYSTEM_MESSAGE_VALUE.toInt())
    val ChatBubble: Color            = Color(CHAT_BUBBLE_VALUE.toInt())   // T-30052: 聊天气泡背景
}
