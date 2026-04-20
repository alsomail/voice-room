package com.voice.room.android.core.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import java.util.Locale

/**
 * darkColorScheme 映射 — MenaColors → Material3 色值
 */
private val MenaDarkColorScheme = darkColorScheme(
    background = MenaColors.Background,
    surface = MenaColors.Surface,
    surfaceVariant = MenaColors.SurfaceVariant,
    primary = MenaColors.Primary,
    onBackground = MenaColors.OnBackground,
    onSurface = MenaColors.OnBackground,
    error = MenaColors.Error,
)

/**
 * MenaTheme — 中东黑金主题入口 Composable
 *
 * 功能:
 * 1. 始终使用 darkColorScheme（无论系统是否深色模式）
 * 2. 注入 MenaTypography 排版规范
 * 3. 注入 MenaShapes 形状规范
 * 4. 根据系统 Locale 自动设置 RTL / LTR 布局方向
 */
@Composable
fun MenaTheme(
    content: @Composable () -> Unit,
) {
    // 根据当前 Locale 决定布局方向
    val configuration = LocalConfiguration.current
    val locale = configuration.locales.get(0) ?: Locale.getDefault()
    val layoutDirection = if (isRtlLocale(locale)) {
        LayoutDirection.Rtl
    } else {
        LayoutDirection.Ltr
    }

    CompositionLocalProvider(LocalLayoutDirection provides layoutDirection) {
        MaterialTheme(
            colorScheme = MenaDarkColorScheme,
            typography = MenaTypography,
            shapes = MenaShapes,
            content = content,
        )
    }
}

/**
 * 判断给定 Locale 是否为 RTL 语言
 */
private fun isRtlLocale(locale: Locale): Boolean {
    val directionality = Character.getDirectionality(locale.displayName.firstOrNull() ?: return false)
    return directionality == Character.DIRECTIONALITY_RIGHT_TO_LEFT ||
            directionality == Character.DIRECTIONALITY_RIGHT_TO_LEFT_ARABIC
}
