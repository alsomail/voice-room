package com.voice.room.android.feature.room

import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI tests for MicSlotCard — T-30011 验收用例 MC-01~MC-12
 *
 * 运行于真机/模拟器（androidTest）。
 * CI 环境无真实设备，仅验证编译通过（compileDebugAndroidTestKotlin）。
 */
@RunWith(AndroidJUnit4::class)
class MicSlotCardTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    // ── 辅助构造 ──────────────────────────────────────────────────────────────

    private fun emptySlot(index: Int = 0) = MicSlotUi(
        index = index, userId = null, nickname = null, avatarUrl = null, isMuted = false
    )

    private fun occupiedSlot(index: Int = 0) = MicSlotUi(
        index = index, userId = "u1", nickname = "Alice", avatarUrl = null, isMuted = false
    )

    private fun mutedSlot(index: Int = 0) = MicSlotUi(
        index = index, userId = "u2", nickname = "Bob", avatarUrl = null, isMuted = true
    )

    // ── MC-01: EMPTY 态 ────────────────────────────────────────────────────────

    /**
     * MC-01: EMPTY 状态正确渲染
     * - testTag("mic_slot_empty_2") 可见
     * - 1-based 座位序号 "3" 可见
     * - testTag("mic_slot_occupied_2") 不存在
     */
    @Test
    fun MC_01_empty_state_renders_correctly() {
        composeTestRule.setContent {
            MicSlotCard(slot = emptySlot(index = 2))
        }
        composeTestRule.onNodeWithTag("mic_slot_empty_2").assertIsDisplayed()
        composeTestRule.onNodeWithText("3").assertIsDisplayed()       // 1-based
        composeTestRule.onNodeWithTag("mic_slot_occupied_2").assertDoesNotExist()
    }

    // ── MC-02: OCCUPIED 态 ────────────────────────────────────────────────────

    /**
     * MC-02: OCCUPIED 状态正确渲染
     * - testTag("mic_slot_occupied_0") 可见
     * - 昵称 "Alice" 可见
     * - 禁麦图标不存在
     * - 音浪动画占位可见
     */
    @Test
    fun MC_02_occupied_state_renders_correctly() {
        composeTestRule.setContent {
            MicSlotCard(slot = occupiedSlot(index = 0))
        }
        composeTestRule.waitForIdle()
        
        composeTestRule.onNodeWithTag("mic_slot_occupied_0").assertIsDisplayed()
        // Round 3 BUG-002：昵称 Text 在 clickable 容器内部（未被合并），
        // 改用 useUnmergedTree=true 查找
        composeTestRule.onNodeWithText("Alice", useUnmergedTree = true).assertIsDisplayed()
        composeTestRule.onNodeWithTag("mic_slot_muted_icon_0", useUnmergedTree = true).assertDoesNotExist()
        
        // Round 3 BUG-002：mic_slot_sound_wave 在 AnimatedVisibility 内部，
        // 需要等待进入动画完成（最长 160ms + 额外 buffer = 500ms）
        composeTestRule.waitUntil(timeoutMillis = 1000) {
            composeTestRule
                .onAllNodesWithTag("mic_slot_sound_wave", useUnmergedTree = true)
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("mic_slot_sound_wave", useUnmergedTree = true).assertIsDisplayed()
    }

    // ── MC-03: MUTED 态 ───────────────────────────────────────────────────────

    /**
     * MC-03: MUTED 状态正确渲染
     * - testTag("mic_slot_occupied_1") 可见
     * - 昵称 "Bob" 可见
     * - 禁麦图标 testTag("mic_slot_muted_icon_1") 可见
     * - 音浪动画不存在
     */
    @Test
    fun MC_03_muted_state_renders_correctly() {
        composeTestRule.setContent {
            MicSlotCard(slot = mutedSlot(index = 1))
        }
        composeTestRule.onNodeWithTag("mic_slot_occupied_1").assertIsDisplayed()
        // Round 3 BUG-002：昵称 / muted_icon / sound_wave 在 clickable 容器内部，
        // 改用 useUnmergedTree=true 查找
        composeTestRule.onNodeWithText("Bob", useUnmergedTree = true).assertIsDisplayed()
        composeTestRule.onNodeWithTag("mic_slot_muted_icon_1", useUnmergedTree = true).assertIsDisplayed()
        composeTestRule.onNodeWithTag("mic_slot_sound_wave", useUnmergedTree = true).assertDoesNotExist()
    }

    // ── MC-04: 有人麦位点击 ───────────────────────────────────────────────────

    /**
     * MC-04: 点击有人麦位，onClick 以正确 index 调用
     */
    @Test
    fun MC_04_click_occupied_slot_triggers_callback() {
        var clickedIndex = -1
        composeTestRule.setContent {
            MicSlotCard(slot = occupiedSlot(index = 3)) { index -> clickedIndex = index }
        }
        composeTestRule.onNodeWithTag("mic_slot_occupied_3").performClick()
        assert(clickedIndex == 3) { "Expected 3 but got $clickedIndex" }
    }

    // ── MC-05: 空麦位点击 ─────────────────────────────────────────────────────

    /**
     * MC-05: 点击空麦位，onClick 以正确 index 调用
     */
    @Test
    fun MC_05_click_empty_slot_triggers_callback() {
        var clickedIndex = -1
        composeTestRule.setContent {
            MicSlotCard(slot = emptySlot(index = 0)) { index -> clickedIndex = index }
        }
        composeTestRule.onNodeWithTag("mic_slot_empty_0").performClick()
        assert(clickedIndex == 0) { "Expected 0 but got $clickedIndex" }
    }

    // ── MC-08: EMPTY 无障碍语义 ───────────────────────────────────────────────

    /**
     * MC-08: EMPTY 麦位无障碍 contentDescription = "麦位 5，空位，点击上麦"
     */
    @Test
    fun MC_08_empty_slot_accessibility_description() {
        composeTestRule.setContent {
            MicSlotCard(slot = emptySlot(index = 4))
        }
        // Round 3 BUG-002 修复：contentDescription 来自 R.string.mic_slot_empty_desc（英文）
        composeTestRule
            .onNodeWithContentDescription("Mic seat 5, empty, tap to take seat")
            .assertExists()
    }

    // ── MC-09: OCCUPIED 无障碍语义 ───────────────────────────────────────────

    /**
     * MC-09: OCCUPIED 麦位无障碍 contentDescription = "麦位 1，Alice，点击互动"
     */
    @Test
    fun MC_09_occupied_slot_accessibility_description() {
        composeTestRule.setContent {
            MicSlotCard(slot = occupiedSlot(index = 0))
        }
        // Round 3 BUG-002 修复：contentDescription 来自 R.string.mic_slot_occupied_desc（英文）
        composeTestRule
            .onNodeWithContentDescription("Mic seat 1, Alice, tap to interact")
            .assertExists()
    }

    // ── MC-10: MUTED 无障碍语义 ───────────────────────────────────────────────

    /**
     * MC-10: MUTED 麦位无障碍 contentDescription = "麦位 1，Bob，已禁麦"
     */
    @Test
    fun MC_10_muted_slot_accessibility_description() {
        composeTestRule.setContent {
            MicSlotCard(
                slot = MicSlotUi(
                    index = 0, userId = "u1", nickname = "Bob",
                    avatarUrl = null, isMuted = true
                )
            )
        }
        // Round 3 BUG-002 修复：contentDescription 来自 R.string.mic_slot_muted_desc（英文）
        composeTestRule
            .onNodeWithContentDescription("Mic seat 1, Bob, muted")
            .assertExists()
    }

    // ── MC-11: MicSlotsGrid 点击透传 ─────────────────────────────────────────

    /**
     * MC-11: MicSlotsGrid 中点击 index=2 的麦位 → onMicSlotClick(2) 被调用
     */
    @Test
    fun MC_11_mic_slots_grid_click_propagates_correct_index() {
        var receivedIndex = -1
        val slots = List(9) { MicSlotUi(index = it) }   // 全部空麦
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots, onMicSlotClick = { index -> receivedIndex = index })
        }
        composeTestRule.onNodeWithTag("mic_slot_empty_2").performClick()
        assert(receivedIndex == 2) { "Expected 2 but got $receivedIndex" }
    }

    // ── MC-12: OCCUPIED → MUTED 时音浪动画消失（AnimatedVisibility） ──────────

    /**
     * MC-12: 当麦位从 OCCUPIED 切换为 MUTED 时，
     *        音浪动画节点（mic_slot_sound_wave）通过 AnimatedVisibility 退出。
     *
     * 验证：初始 OCCUPIED 时音浪可见；切换 isMuted=true 后音浪不显示。
     * （androidTest 仅验证编译通过；设备上执行 exit 动画后节点消失）
     */
    @Test
    fun MC_12_sound_wave_hidden_when_muted_via_animated_visibility() {
        val slotState = mutableStateOf(
            MicSlotUi(index = 0, userId = "u1", nickname = "Alice", avatarUrl = null, isMuted = false)
        )
        composeTestRule.setContent {
            val slot = remember { slotState }
            MicSlotCard(slot = slot.value)
        }
        composeTestRule.waitForIdle()
        
        // Round 3 BUG-002：mic_slot_sound_wave 在 AnimatedVisibility 内部，需 useUnmergedTree
        // OCCUPIED 时音浪可见，需等待进入动画完成
        composeTestRule.waitUntil(timeoutMillis = 1000) {
            composeTestRule
                .onAllNodesWithTag("mic_slot_sound_wave", useUnmergedTree = true)
                .fetchSemanticsNodes().isNotEmpty()
        }
        composeTestRule.onNodeWithTag("mic_slot_sound_wave", useUnmergedTree = true).assertIsDisplayed()

        // 切换为 MUTED
        composeTestRule.runOnIdle {
            slotState.value = slotState.value.copy(isMuted = true)
        }
        // AnimatedVisibility 退出动画结束后，节点不再显示
        composeTestRule.mainClock.autoAdvance = false
        composeTestRule.mainClock.advanceTimeBy(500L)
        composeTestRule.onNodeWithTag("mic_slot_sound_wave", useUnmergedTree = true).assertDoesNotExist()
        composeTestRule.mainClock.autoAdvance = true
    }
}
