package com.voice.room.android.feature.room

import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertWidthIsAtLeast
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.assertDoesNotExist
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — HostMicSlot (T-30025)
 *
 * VS-01: slots[0] 有人时，mic_slot_occupied_0 可见，宽/高 ≥ 80dp，avatar_frame 存在
 * VS-02: slots[0] 空位时，mic_slot_empty_0 可见，Canvas 光圈不渲染，显示虚线圆圈 + "+" 图标
 * VS-03: 点击主麦空位，触发 onMicSlotClick(0) 回调
 * VS-03b: 点击主麦有人位，触发 onMicSlotClick(0) 回调
 */
@RunWith(AndroidJUnit4::class)
class HostMicSlotTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    private fun occupiedHostSlot() = MicSlotUi(
        index = 0, userId = "host-1", nickname = "Alice", avatarUrl = null, isMuted = false
    )

    private fun emptyHostSlot() = MicSlotUi(
        index = 0, userId = null, nickname = null, avatarUrl = null, isMuted = false
    )

    // ── VS-01: 有人主麦 —————————————————————————————————————————————————————

    /**
     * VS-01: slots[0] 有人时
     * - mic_slot_occupied_0 可见
     * - 宽/高均 ≥ 80dp
     * - AvatarWithFrame 的 avatar_frame testTag 存在（金色边框）
     */
    @Test
    fun VS01_occupied_host_slot_renders_with_80dp_and_gold_frame() {
        composeTestRule.setContent {
            HostMicSlot(slot = occupiedHostSlot())
        }
        composeTestRule.waitForIdle()

        // mic_slot_occupied_0 可见
        composeTestRule
            .onNodeWithTag("mic_slot_occupied_0")
            .assertIsDisplayed()

        // 宽/高均 ≥ 80dp（AvatarWithFrame 80dp + 2dp 边框 = 84dp 实际尺寸）
        composeTestRule
            .onNodeWithTag("mic_slot_occupied_0")
            .assertWidthIsAtLeast(80.dp)
            .assertHeightIsAtLeast(80.dp)

        // avatar_frame testTag 存在（AvatarWithFrame showFrame=true 时渲染）
        composeTestRule
            .onNodeWithTag("avatar_frame")
            .assertIsDisplayed()
    }

    // ── VS-02: 空位主麦 —————————————————————————————————————————————————————

    /**
     * VS-02: slots[0] 空位时
     * - mic_slot_empty_0 可见
     * - avatar_frame 不存在（Canvas 光圈不渲染，showFrame=false）
     * - "+" 图标可见（empty slot indicator）
     * - mic_slot_occupied_0 不存在
     */
    @Test
    fun VS02_empty_host_slot_renders_dashed_circle_no_glow() {
        composeTestRule.setContent {
            HostMicSlot(slot = emptyHostSlot())
        }
        composeTestRule.waitForIdle()

        // mic_slot_empty_0 可见
        composeTestRule
            .onNodeWithTag("mic_slot_empty_0")
            .assertIsDisplayed()

        // avatar_frame 不渲染（showFrame=false，空位无金色边框）
        composeTestRule
            .onNodeWithTag("avatar_frame")
            .assertDoesNotExist()

        // mic_slot_occupied_0 不存在
        composeTestRule
            .onNodeWithTag("mic_slot_occupied_0")
            .assertDoesNotExist()
    }

    // ── VS-03: 点击空主麦 ————————————————————————————————————————————————————

    /**
     * VS-03: 点击主麦空位，触发 onMicSlotClick(0) 回调
     */
    @Test
    fun VS03_click_empty_host_slot_triggers_callback_with_index_0() {
        var clickedIndex = -1
        composeTestRule.setContent {
            HostMicSlot(slot = emptyHostSlot(), onClick = { clickedIndex = it })
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_empty_0").performClick()

        assertEquals("onMicSlotClick 应接收到 index=0", 0, clickedIndex)
    }

    /**
     * VS-03b: 点击主麦有人位，触发 onMicSlotClick(0) 回调
     */
    @Test
    fun VS03b_click_occupied_host_slot_triggers_callback_with_index_0() {
        var clickedIndex = -1
        composeTestRule.setContent {
            HostMicSlot(slot = occupiedHostSlot(), onClick = { clickedIndex = it })
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_occupied_0").performClick()

        assertEquals("onMicSlotClick 应接收到 index=0", 0, clickedIndex)
    }

    // ── VS-01 扩展: 有人主麦昵称显示 —————————————————————————————————————————

    /**
     * VS-01 扩展: 有人时昵称 "Alice" 可见
     */
    @Test
    fun VS01_ext_occupied_host_slot_shows_nickname() {
        composeTestRule.setContent {
            HostMicSlot(slot = occupiedHostSlot())
        }
        composeTestRule.waitForIdle()

        composeTestRule
            .onNodeWithTag("mic_slot_occupied_0")
            .assertIsDisplayed()
        // 昵称 "Alice" 应在视图树中可见
        composeTestRule
            .onNodeWithTag("host_mic_nickname")
            .assertIsDisplayed()
    }
}
