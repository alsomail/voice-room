package com.voice.room.android.feature.room.create

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — CoverPickerBottomSheet 状态逻辑 (T-30037)
 *
 * CP37-01: 默认选中第一张（selectedUrl == COVER_OPTIONS[0].url）
 * CP37-02: 点击某项 → selectCover(index) 更新 selectedUrl
 * CP37-03: 选中项 selectedIndex 与操作一致（金色边框状态正确）
 * CP37-04: confirmSelection() 触发 onCoverSelected(selectedUrl) 回调
 * CP37-05: COVER_OPTIONS 共有 8 张封面
 *
 * 说明：Compose UI 渲染层在 Instrumented Test 中验证；
 *       本文件使用纯 JVM 测试 [CoverPickerState] 状态持有者，
 *       不依赖 Android Framework / Compose Runtime。
 */
class CoverPickerBottomSheetTest {

    // ─────────────────────────────────────────────
    // CP37-05: COVER_OPTIONS 共有 8 张封面
    // ─────────────────────────────────────────────

    @Test
    fun CP37_05_coverOptions_hasTotalEightItems() {
        assertEquals(
            "COVER_OPTIONS should contain exactly 8 preset covers",
            8,
            COVER_OPTIONS.size
        )
    }

    @Test
    fun CP37_05_coverOptions_allUrlsAreNonEmpty() {
        COVER_OPTIONS.forEachIndexed { index, option ->
            assertTrue(
                "COVER_OPTIONS[$index].url should not be blank, actual='${option.url}'",
                option.url.isNotBlank()
            )
        }
    }

    @Test
    fun CP37_05_coverOptions_urlsMatchExpectedPaths() {
        val expectedUrls = listOf(
            "/assets/covers/desert.webp",
            "/assets/covers/mosque.webp",
            "/assets/covers/lantern.webp",
            "/assets/covers/eagle.webp",
            "/assets/covers/rose.webp",
            "/assets/covers/yacht.webp",
            "/assets/covers/sunset.webp",
            "/assets/covers/calligraphy.webp",
        )
        assertEquals(
            "COVER_OPTIONS URLs should match expected server paths",
            expectedUrls,
            COVER_OPTIONS.map { it.url }
        )
    }

    @Test
    fun CP37_05_coverOptions_allUrlsAreUnique() {
        val urls = COVER_OPTIONS.map { it.url }
        assertEquals(
            "All cover URLs should be unique (no duplicates)",
            urls.distinct().size,
            urls.size
        )
    }

    // ─────────────────────────────────────────────
    // CP37-01: 默认选中第一张
    // ─────────────────────────────────────────────

    @Test
    fun CP37_01_initialSelectedUrl_isFirstCoverOption() {
        val state = CoverPickerState(onCoverSelected = {})

        assertEquals(
            "Default selectedUrl should be COVER_OPTIONS[0].url",
            COVER_OPTIONS[0].url,
            state.selectedUrl
        )
    }

    @Test
    fun CP37_01_initialSelectedIndex_isZero() {
        val state = CoverPickerState(onCoverSelected = {})

        assertEquals(
            "Default selectedIndex should be 0",
            0,
            state.selectedIndex
        )
    }

    // ─────────────────────────────────────────────
    // CP37-02: 点击某项更新选中态
    // ─────────────────────────────────────────────

    @Test
    fun CP37_02_selectCover_updatesSelectedUrl() {
        val state = CoverPickerState(onCoverSelected = {})

        state.selectCover(3)

        assertEquals(
            "selectedUrl should update to COVER_OPTIONS[3].url after selectCover(3)",
            COVER_OPTIONS[3].url,
            state.selectedUrl
        )
    }

    @Test
    fun CP37_02_selectCover_fromFirstToLast_updatesCorrectly() {
        val state = CoverPickerState(onCoverSelected = {})

        // 默认是 index=0，切换到最后一张 index=7
        state.selectCover(7)

        assertEquals(
            "selectedUrl should update to COVER_OPTIONS[7].url",
            COVER_OPTIONS[7].url,
            state.selectedUrl
        )
        assertNotEquals(
            "selectedUrl must have changed from initial value",
            COVER_OPTIONS[0].url,
            state.selectedUrl
        )
    }

    @Test
    fun CP37_02_selectCover_sequentialSelections_alwaysReflectsLatest() {
        val state = CoverPickerState(onCoverSelected = {})

        state.selectCover(1)
        state.selectCover(5)
        state.selectCover(2)

        assertEquals(
            "selectedUrl should reflect the last selectCover(2) call",
            COVER_OPTIONS[2].url,
            state.selectedUrl
        )
    }

    // ─────────────────────────────────────────────
    // CP37-03: 选中项有金色边框（selectedIndex 正确）
    // ─────────────────────────────────────────────

    @Test
    fun CP37_03_selectedIndex_matchesSelectCoverIndex() {
        val state = CoverPickerState(onCoverSelected = {})

        for (i in 0 until COVER_OPTIONS.size) {
            state.selectCover(i)
            assertEquals(
                "selectedIndex should be $i after selectCover($i)",
                i,
                state.selectedIndex
            )
        }
    }

    @Test
    fun CP37_03_goldBorderColor_isCorrectValue() {
        // 金色边框色值常量验证：0xFFD4AF37
        val expectedGoldArgb = 0xFFD4AF37L
        assertEquals(
            "COVER_GOLD_BORDER_COLOR should be 0xFFD4AF37",
            expectedGoldArgb,
            COVER_GOLD_BORDER_COLOR
        )
    }

    @Test
    fun CP37_03_goldBorderWidth_isTwoDp() {
        assertEquals(
            "COVER_GOLD_BORDER_WIDTH_DP should be 2",
            2,
            COVER_GOLD_BORDER_WIDTH_DP
        )
    }

    // ─────────────────────────────────────────────
    // CP37-04: 确认后回调正确 url
    // ─────────────────────────────────────────────

    @Test
    fun CP37_04_confirmSelection_invokesCallbackWithSelectedUrl() {
        var callbackUrl: String? = null
        val state = CoverPickerState(onCoverSelected = { url -> callbackUrl = url })

        state.selectCover(4)
        state.confirmSelection()

        assertEquals(
            "onCoverSelected callback should receive COVER_OPTIONS[4].url",
            COVER_OPTIONS[4].url,
            callbackUrl
        )
    }

    @Test
    fun CP37_04_confirmSelection_withDefaultSelection_callbackReceivesFirstUrl() {
        var callbackUrl: String? = null
        val state = CoverPickerState(onCoverSelected = { url -> callbackUrl = url })

        // 不做任何选择，直接确认
        state.confirmSelection()

        assertEquals(
            "Confirming without changing selection should callback with COVER_OPTIONS[0].url",
            COVER_OPTIONS[0].url,
            callbackUrl
        )
    }

    @Test
    fun CP37_04_confirmSelection_callbackIsInvokedExactlyOnce() {
        var callCount = 0
        val state = CoverPickerState(onCoverSelected = { callCount++ })

        state.confirmSelection()

        assertEquals(
            "onCoverSelected callback should be invoked exactly once per confirmSelection()",
            1,
            callCount
        )
    }

    @Test
    fun CP37_04_confirmSelection_multipleConfirms_callbackInvokedEachTime() {
        val receivedUrls = mutableListOf<String>()
        val state = CoverPickerState(onCoverSelected = { url -> receivedUrls.add(url) })

        state.selectCover(0)
        state.confirmSelection()
        state.selectCover(2)
        state.confirmSelection()

        assertEquals("Two confirmations should result in two callbacks", 2, receivedUrls.size)
        assertEquals(COVER_OPTIONS[0].url, receivedUrls[0])
        assertEquals(COVER_OPTIONS[2].url, receivedUrls[1])
    }

    // ─────────────────────────────────────────────
    // 边界情况
    // ─────────────────────────────────────────────

    @Test
    fun boundaryCase_selectCover_indexZero_isFirstOption() {
        val state = CoverPickerState(onCoverSelected = {})
        state.selectCover(0)
        assertEquals(COVER_OPTIONS[0].url, state.selectedUrl)
    }

    @Test
    fun boundaryCase_selectCover_lastIndex_isLastOption() {
        val state = CoverPickerState(onCoverSelected = {})
        state.selectCover(COVER_OPTIONS.size - 1)
        assertEquals(COVER_OPTIONS.last().url, state.selectedUrl)
    }

    @Test
    fun boundaryCase_customInitialUrl_isReflectedInState() {
        val customUrl = COVER_OPTIONS[5].url
        val state = CoverPickerState(
            initialUrl = customUrl,
            onCoverSelected = {}
        )

        assertEquals(
            "CoverPickerState with custom initialUrl should start with that url",
            customUrl,
            state.selectedUrl
        )
        assertEquals(
            "selectedIndex should match the initial url index (5)",
            5,
            state.selectedIndex
        )
    }
}
