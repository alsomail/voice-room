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

    // ── Compose Color 常量 ────────────────────────
    val Background: Color            = Color(BACKGROUND_VALUE)
    val Surface: Color               = Color(SURFACE_VALUE)
    val SurfaceVariant: Color        = Color(SURFACE_VARIANT_VALUE)
    val Primary: Color               = Color(PRIMARY_VALUE)
    val PrimaryBright: Color         = Color(PRIMARY_BRIGHT_VALUE)
    val OnBackground: Color          = Color(ON_BACKGROUND_VALUE)
    val OnBackgroundSecondary: Color = Color(ON_BACKGROUND_SECONDARY_VALUE)
    val OnBackgroundTertiary: Color  = Color(ON_BACKGROUND_TERTIARY_VALUE)
    val Error: Color                 = Color(ERROR_VALUE)
    val Success: Color               = Color(SUCCESS_VALUE)
    val SystemMessage: Color         = Color(SYSTEM_MESSAGE_VALUE)
}
