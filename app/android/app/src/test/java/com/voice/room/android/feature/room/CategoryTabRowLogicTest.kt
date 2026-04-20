package com.voice.room.android.feature.room

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM 单元测试 — CategoryTabRow 数据逻辑验证 (T-30022)
 *
 * V-07: 分类横滑显示 "热门" tab 且默认选中
 * 验证 tab 列表常量和启用/禁用逻辑。
 */
class CategoryTabRowLogicTest {

    // ─────────────────────────────────────────────
    // V-07: Tab 列表为 ["热门","新开","关注","游戏"]
    // ─────────────────────────────────────────────

    @Test
    fun `V07 category tabs contains 4 items with correct labels`() {
        assertEquals(
            listOf("热门", "新开", "关注", "游戏"),
            CATEGORY_TABS
        )
    }

    // ─────────────────────────────────────────────
    // V-07: 默认选中 index = 0 ("热门")
    // ─────────────────────────────────────────────

    @Test
    fun `V07 default selected index is 0`() {
        assertEquals(0, DEFAULT_SELECTED_TAB_INDEX)
    }

    // ─────────────────────────────────────────────
    // Phase 0.5: 仅 index=0 可交互，其余 disabled
    // ─────────────────────────────────────────────

    @Test
    fun `only index 0 is enabled in phase 0_5`() {
        assertTrue("Index 0 should be enabled", isCategoryTabEnabled(0))
        assertFalse("Index 1 should be disabled", isCategoryTabEnabled(1))
        assertFalse("Index 2 should be disabled", isCategoryTabEnabled(2))
        assertFalse("Index 3 should be disabled", isCategoryTabEnabled(3))
    }
}
