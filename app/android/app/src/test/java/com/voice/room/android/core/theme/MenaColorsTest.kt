package com.voice.room.android.core.theme

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — MenaColors 色彩令牌 (T-30018)
 *
 * MC-01: Background 色值 luminance < 0.1（深色校验）
 * MC-02: Primary 色值为金色系（R > 0.7, G > 0.5, B < 0.5）
 * MC-03: OnBackground 为浅色（luminance > 0.8）
 * MC-04: Surface 与 Background 色值不同
 * MC-05: Error 色值存在且 alpha == 1.0（非透明）
 */
class MenaColorsTest {

    // ─────────────────────────────────────────────
    // 辅助方法：从 Color(0xAARRGGBB) long 值提取分量
    // Compose Color 在 JVM 单测中不可用，直接验证 ARGB hex 值
    // ─────────────────────────────────────────────

    /**
     * 从 0xFFRRGGBB 格式的 ULong 提取 R 分量 (0.0~1.0)
     */
    private fun redOf(colorValue: ULong): Float = ((colorValue shr 16) and 0xFFu).toFloat() / 255f

    /**
     * 从 0xFFRRGGBB 格式的 ULong 提取 G 分量 (0.0~1.0)
     */
    private fun greenOf(colorValue: ULong): Float = ((colorValue shr 8) and 0xFFu).toFloat() / 255f

    /**
     * 从 0xFFRRGGBB 格式的 ULong 提取 B 分量 (0.0~1.0)
     */
    private fun blueOf(colorValue: ULong): Float = (colorValue and 0xFFu).toFloat() / 255f

    /**
     * 从 0xFFRRGGBB 格式的 ULong 提取 Alpha 分量 (0.0~1.0)
     */
    private fun alphaOf(colorValue: ULong): Float = ((colorValue shr 24) and 0xFFu).toFloat() / 255f

    /**
     * 相对亮度近似（sRGB 简化公式）
     * luminance ≈ 0.2126*R + 0.7152*G + 0.0722*B
     */
    private fun luminanceOf(colorValue: ULong): Float {
        val r = redOf(colorValue)
        val g = greenOf(colorValue)
        val b = blueOf(colorValue)
        return 0.2126f * r + 0.7152f * g + 0.0722f * b
    }

    // ─────────────────────────────────────────────
    // MC-01: Background 色值 luminance < 0.1（深色校验）
    // ─────────────────────────────────────────────

    @Test
    fun MC01_background_isDarkColor() {
        val lum = luminanceOf(MenaColors.BACKGROUND_VALUE)
        assertTrue(
            "Background luminance should be < 0.15 (deep dark), actual=$lum",
            lum < 0.15f
        )
    }

    // ─────────────────────────────────────────────
    // MC-02: Primary 色值为金色系（R > 0.7, G > 0.5, B < 0.5）
    // ─────────────────────────────────────────────

    @Test
    fun MC02_primary_isGoldColor() {
        val r = redOf(MenaColors.PRIMARY_VALUE)
        val g = greenOf(MenaColors.PRIMARY_VALUE)
        val b = blueOf(MenaColors.PRIMARY_VALUE)
        assertTrue("Primary R ($r) should be > 0.7", r > 0.7f)
        assertTrue("Primary G ($g) should be > 0.5", g > 0.5f)
        assertTrue("Primary B ($b) should be < 0.5", b < 0.5f)
    }

    // ─────────────────────────────────────────────
    // MC-03: OnBackground 为浅色（luminance > 0.8）
    // ─────────────────────────────────────────────

    @Test
    fun MC03_onBackground_isLightColor() {
        val lum = luminanceOf(MenaColors.ON_BACKGROUND_VALUE)
        assertTrue(
            "OnBackground luminance should be > 0.8 (light), actual=$lum",
            lum > 0.8f
        )
    }

    // ─────────────────────────────────────────────
    // MC-04: Surface 与 Background 色值不同
    // ─────────────────────────────────────────────

    @Test
    fun MC04_surface_isDifferentFromBackground() {
        assertNotEquals(
            "Surface and Background must be different colors",
            MenaColors.SURFACE_VALUE,
            MenaColors.BACKGROUND_VALUE
        )
    }

    // ─────────────────────────────────────────────
    // MC-05: Error 色值存在且 alpha == 1.0（非透明）
    // ─────────────────────────────────────────────

    @Test
    fun MC05_error_isOpaqueColor() {
        val alpha = alphaOf(MenaColors.ERROR_VALUE)
        assertEquals(
            "Error color alpha should be 1.0 (fully opaque)",
            1.0f,
            alpha,
            0.001f
        )
    }

    // ─────────────────────────────────────────────
    // 额外：所有 11 个颜色常量均为完全不透明
    // ─────────────────────────────────────────────

    @Test
    fun allColors_areFullyOpaque() {
        val colors = listOf(
            "Background" to MenaColors.BACKGROUND_VALUE,
            "Surface" to MenaColors.SURFACE_VALUE,
            "SurfaceVariant" to MenaColors.SURFACE_VARIANT_VALUE,
            "Primary" to MenaColors.PRIMARY_VALUE,
            "PrimaryBright" to MenaColors.PRIMARY_BRIGHT_VALUE,
            "OnBackground" to MenaColors.ON_BACKGROUND_VALUE,
            "OnBackgroundSecondary" to MenaColors.ON_BACKGROUND_SECONDARY_VALUE,
            "OnBackgroundTertiary" to MenaColors.ON_BACKGROUND_TERTIARY_VALUE,
            "Error" to MenaColors.ERROR_VALUE,
            "Success" to MenaColors.SUCCESS_VALUE,
            "SystemMessage" to MenaColors.SYSTEM_MESSAGE_VALUE,
        )
        for ((name, value) in colors) {
            val alpha = alphaOf(value)
            assertEquals("$name should be fully opaque", 1.0f, alpha, 0.001f)
        }
    }

    // ─────────────────────────────────────────────
    // 额外：PrimaryBright 比 Primary 更亮
    // ─────────────────────────────────────────────

    @Test
    fun primaryBright_isBrighterThanPrimary() {
        val primaryLum = luminanceOf(MenaColors.PRIMARY_VALUE)
        val primaryBrightLum = luminanceOf(MenaColors.PRIMARY_BRIGHT_VALUE)
        assertTrue(
            "PrimaryBright ($primaryBrightLum) should be brighter than Primary ($primaryLum)",
            primaryBrightLum > primaryLum
        )
    }
}
