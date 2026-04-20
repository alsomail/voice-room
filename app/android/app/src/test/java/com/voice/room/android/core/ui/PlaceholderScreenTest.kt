package com.voice.room.android.core.ui

import com.voice.room.android.core.theme.MenaColors
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM 单元测试 — PlaceholderScreen 视觉规范校验 (T-30023)
 *
 * PH-05: 标题颜色 = MenaColors.OnBackgroundSecondary (#B0B0B0)
 * 图标颜色 = MenaColors.OnBackgroundTertiary (#6C6C6C)
 * 副标题颜色 = MenaColors.OnBackgroundTertiary (#6C6C6C)
 *
 * 这些测试验证 PlaceholderScreen 设计规范中引用的色值常量，
 * 确保色值与设计文档一致，避免无意修改导致视觉回归。
 */
class PlaceholderScreenTest {

    // ─────────────────────────────────────────────
    // PH-05: 标题文字颜色为 MenaColors.OnBackgroundSecondary
    // ─────────────────────────────────────────────

    @Test
    fun PH05_titleColor_matchesOnBackgroundSecondary_B0B0B0() {
        assertEquals(
            "Title color should be OnBackgroundSecondary (#B0B0B0)",
            0xFFB0B0B0uL,
            MenaColors.ON_BACKGROUND_SECONDARY_VALUE
        )
    }

    @Test
    fun PH05_iconColor_matchesOnBackgroundTertiary_6C6C6C() {
        assertEquals(
            "Icon tint should be OnBackgroundTertiary (#6C6C6C)",
            0xFF6C6C6CuL,
            MenaColors.ON_BACKGROUND_TERTIARY_VALUE
        )
    }

    @Test
    fun PH05_subtitleColor_matchesOnBackgroundTertiary_6C6C6C() {
        assertEquals(
            "Subtitle color should be OnBackgroundTertiary (#6C6C6C)",
            0xFF6C6C6CuL,
            MenaColors.ON_BACKGROUND_TERTIARY_VALUE
        )
    }

    // ─────────────────────────────────────────────
    // PlaceholderScreen 设计规范常量校验
    // ─────────────────────────────────────────────

    @Test
    fun designSpec_iconSize_is64dp() {
        assertEquals(
            "Icon size should be 64dp per design spec",
            64,
            PlaceholderScreenDefaults.ICON_SIZE_DP
        )
    }

    @Test
    fun designSpec_iconToTitleSpacing_is8dp() {
        assertEquals(
            "Icon-to-title spacing should be 8dp per design spec",
            8,
            PlaceholderScreenDefaults.ICON_TO_TITLE_SPACING_DP
        )
    }

    @Test
    fun designSpec_titleToSubtitleSpacing_is4dp() {
        assertEquals(
            "Title-to-subtitle spacing should be 4dp per design spec",
            4,
            PlaceholderScreenDefaults.TITLE_TO_SUBTITLE_SPACING_DP
        )
    }

    // ─────────────────────────────────────────────
    // 背景色深色校验（与 Scaffold containerColor 一致）
    // ─────────────────────────────────────────────

    @Test
    fun designSpec_background_isDarkColor() {
        // MenaColors.Background (#1A1A2E) luminance must be < 0.15
        val r = ((MenaColors.BACKGROUND_VALUE shr 16) and 0xFFu).toFloat() / 255f
        val g = ((MenaColors.BACKGROUND_VALUE shr 8) and 0xFFu).toFloat() / 255f
        val b = (MenaColors.BACKGROUND_VALUE and 0xFFu).toFloat() / 255f
        val luminance = 0.2126f * r + 0.7152f * g + 0.0722f * b
        assertTrue(
            "Background luminance ($luminance) should be < 0.15 (deep dark)",
            luminance < 0.15f
        )
    }
}
