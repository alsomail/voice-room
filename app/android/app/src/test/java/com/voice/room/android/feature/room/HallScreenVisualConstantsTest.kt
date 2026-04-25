package com.voice.room.android.feature.room

import com.voice.room.android.core.theme.MenaColors
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * JVM 单元测试 — 大厅页视觉升级色值常量验证 (T-30022)
 *
 * 验证 MenaColors 中与 HallScreen 视觉升级相关的所有色值常量，
 * 确保 TDS 颜色映射表中的色值在代码层面正确定义。
 *
 * V-01: Surface (#16213E) — RoomCard 底色
 * V-02: SurfaceVariant (#0F3460) — 渐变色块占位区
 * V-05: Primary (#D4AF37) — FAB 金色 + 顶部栏标题
 * V-08: Background (#1A1A2E) — 页面背景
 */
class HallScreenVisualConstantsTest {

    // ─────────────────────────────────────────────
    // V-08: 页面背景色 = #1A1A2E
    // ─────────────────────────────────────────────

    @Test
    fun `V08 background color value matches 1A1A2E`() {
        assertEquals(0xFF1A1A2EuL, MenaColors.BACKGROUND_VALUE)
    }

    // ─────────────────────────────────────────────
    // V-01: RoomCard 底色 = #16213E
    // ─────────────────────────────────────────────

    @Test
    fun `V01 surface color value matches 16213E for RoomCard`() {
        assertEquals(0xFF16213EuL, MenaColors.SURFACE_VALUE)
    }

    // ─────────────────────────────────────────────
    // V-02: 渐变色块区域 = #0F3460
    // ─────────────────────────────────────────────

    @Test
    fun `V02 surfaceVariant color value matches 0F3460 for gradient block`() {
        assertEquals(0xFF0F3460uL, MenaColors.SURFACE_VARIANT_VALUE)
    }

    // ─────────────────────────────────────────────
    // V-05: FAB / 顶部栏金色 = #D4AF37
    // ─────────────────────────────────────────────

    @Test
    fun `V05 primary color value matches D4AF37 for FAB and top bar`() {
        assertEquals(0xFFD4AF37uL, MenaColors.PRIMARY_VALUE)
    }

    // ─────────────────────────────────────────────
    // V-03: 标题文字白色 = #FFFFFF
    // ─────────────────────────────────────────────

    @Test
    fun `V03 onBackground color value matches FFFFFF for title text`() {
        assertEquals(0xFFFFFFFFuL, MenaColors.ON_BACKGROUND_VALUE)
    }

    // ─────────────────────────────────────────────
    // 错误文字色 = #E74C3C
    // ─────────────────────────────────────────────

    @Test
    fun `error color value matches E74C3C`() {
        assertEquals(0xFFE74C3CuL, MenaColors.ERROR_VALUE)
    }

    // ─────────────────────────────────────────────
    // OnBackgroundTertiary = #6C6C6C (未选中 tab)
    // ─────────────────────────────────────────────

    @Test
    fun `onBackgroundTertiary color value matches 6C6C6C for unselected tabs`() {
        assertEquals(0xFF6C6C6CuL, MenaColors.ON_BACKGROUND_TERTIARY_VALUE)
    }
}
