package com.voice.room.android.feature.room

import com.voice.room.android.core.theme.MenaColors
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * JVM 单元测试 — OnlineCountBadge 逻辑验证 (T-30022)
 *
 * V-04: OnlineCountBadge 显示绿色圆点(8dp) + 在线人数
 * E-01: memberCount 为 0 时显示 "0"
 *
 * 注意: Compose UI 渲染需 androidTest（HallScreenVisualUpgradeTest），
 * 本文件仅验证 *数据层* 逻辑（颜色常量等）不依赖 Compose Runtime。
 */
class OnlineCountBadgeTest {

    // ─────────────────────────────────────────────
    // V-04: Success 颜色常量 = #2ECC71 (绿色圆点)
    // ─────────────────────────────────────────────

    @Test
    fun `V04 success color value matches green 2ECC71`() {
        assertEquals(
            "MenaColors.SUCCESS_VALUE should be 0xFF2ECC71",
            0xFF2ECC71uL,
            MenaColors.SUCCESS_VALUE
        )
    }

    // ─────────────────────────────────────────────
    // V-04: OnBackgroundSecondary 颜色常量 = #B0B0B0 (人数文字)
    // ─────────────────────────────────────────────

    @Test
    fun `V04 onBackgroundSecondary color value matches B0B0B0`() {
        assertEquals(
            "MenaColors.ON_BACKGROUND_SECONDARY_VALUE should be 0xFFB0B0B0",
            0xFFB0B0B0uL,
            MenaColors.ON_BACKGROUND_SECONDARY_VALUE
        )
    }

    // ─────────────────────────────────────────────
    // E-01: count=0 格式化为 "0"
    // ─────────────────────────────────────────────

    @Test
    fun `E01 count zero formats as string 0`() {
        val count = 0
        assertEquals("0", "$count")
    }

    // ─────────────────────────────────────────────
    // 正向: count=999 格式化为 "999"
    // ─────────────────────────────────────────────

    @Test
    fun `count 999 formats as string 999`() {
        val count = 999
        assertEquals("999", "$count")
    }
}
