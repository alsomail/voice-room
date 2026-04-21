package com.voice.room.android.feature.room

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.LayoutDirection
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — MicSlotsGrid 黑金风格 (T-30025)
 *
 * VS-04: slots[1..8] 渲染时，副麦为 4 列（GridCells.Fixed(4)）
 * VS-05: 副麦 OCCUPIED 时，mic_slot_occupied_{index} 可见，avatar_frame 存在
 * VS-06: 副麦 EMPTY 时，mic_slot_empty_{index} 可见，不显示麦克风 Icon 文字 "空位"
 * VS-07: 副麦 MUTED 时，mic_slot_muted_icon_{index} 可见
 * VS-08: 空麦位可点击（通过 onClick 回调验证）
 * VS-09: 点击空副麦位，onMicSlotClick(index) 回调 index 与 slot.index 一致
 * VS-18: slots 列表为空时，MicSlotsGrid 不崩溃
 * VS-19: slots 仅 1 个元素（只有主麦），副麦 Grid 渲染 0 项不崩溃
 * VS-20: RTL 布局下，MicSlotsGrid 不崩溃，主麦节点可见
 */
@RunWith(AndroidJUnit4::class)
class MicSlotsGridBlackGoldTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    // ─────────────────────────────────────────────────────────────────────────
    // 辅助函数
    // ─────────────────────────────────────────────────────────────────────────

    private fun emptySlot(index: Int) = MicSlotUi(index = index)
    private fun occupiedSlot(index: Int) = MicSlotUi(
        index = index, userId = "u$index", nickname = "User$index", isMuted = false
    )
    private fun mutedSlot(index: Int) = MicSlotUi(
        index = index, userId = "u$index", nickname = "User$index", isMuted = true
    )

    private fun fullSlots(): List<MicSlotUi> = List(9) { emptySlot(it) }

    // ── VS-04: 副麦 4 列 —————————————————————————————————————————————————————

    /**
     * VS-04: slots[1..8] 渲染时，mic_slots_grid testTag 可见。
     * 4 列配置通过代码审查验证（GridCells.Fixed(4)）；
     * 此测试验证 grid 容器存在且 8 个副麦均渲染。
     */
    @Test
    fun VS04_guest_slots_grid_renders_with_four_columns_tag_visible() {
        val slots = fullSlots()
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        // mic_slots_grid 容器可见
        composeTestRule.onNodeWithTag("mic_slots_grid").assertIsDisplayed()

        // 副麦 slots[1..8] 全部渲染（8 个空麦位）
        for (i in 1..8) {
            composeTestRule.onNodeWithTag("mic_slot_empty_$i").assertIsDisplayed()
        }
    }

    /**
     * VS-04 扩展: 主麦 slot[0] 被单独渲染在 HostMicSlot，不在 mic_slots_grid 内。
     * mic_slot_empty_0 / mic_slot_occupied_0 应在 grid 外的主麦区。
     */
    @Test
    fun VS04_ext_host_slot_rendered_outside_grid() {
        val slots = fullSlots()
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        // 主麦 slot[0] 独立渲染为 HostMicSlot（在 grid 之外）
        composeTestRule.onNodeWithTag("mic_slot_empty_0").assertIsDisplayed()
    }

    // ── VS-05: 副麦 OCCUPIED —————————————————————————————————————————————————

    /**
     * VS-05: 副麦 OCCUPIED 时，mic_slot_occupied_{index} 可见，AvatarWithFrame 的 avatar_frame 存在
     */
    @Test
    fun VS05_guest_occupied_slot_shows_avatar_frame() {
        val slots = listOf(
            emptySlot(0),
            occupiedSlot(1),
        ) + List(7) { emptySlot(it + 2) }

        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        // mic_slot_occupied_1 可见
        composeTestRule.onNodeWithTag("mic_slot_occupied_1").assertIsDisplayed()
        // AvatarWithFrame showFrame=true → avatar_frame testTag 存在
        composeTestRule.onNodeWithTag("avatar_frame").assertIsDisplayed()
    }

    // ── VS-06: 副麦 EMPTY ————————————————————————————————————————————————————

    /**
     * VS-06: 副麦 EMPTY 时，mic_slot_empty_{index} 可见，不显示麦克风 Icon 文字 "空位"
     */
    @Test
    fun VS06_guest_empty_slot_no_mic_icon_no_empty_text() {
        val slots = fullSlots()
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        // mic_slot_empty_3 可见
        composeTestRule.onNodeWithTag("mic_slot_empty_3").assertIsDisplayed()
        // "空位" 文字不显示（旧样式已废弃）
        composeTestRule.onNodeWithText("空位").assertDoesNotExist()
    }

    // ── VS-07: 副麦 MUTED ————————————————————————————————————————————————————

    /**
     * VS-07: 副麦 MUTED 时，mic_slot_muted_icon_{index} 可见
     */
    @Test
    fun VS07_guest_muted_slot_shows_muted_icon() {
        val slots = listOf(
            emptySlot(0),
            emptySlot(1),
            mutedSlot(2),
        ) + List(6) { emptySlot(it + 3) }

        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_occupied_2").assertIsDisplayed()
        composeTestRule.onNodeWithTag("mic_slot_muted_icon_2").assertIsDisplayed()
    }

    // ── VS-08: 空麦位点击回调 ——————————————————————————————————————————————————

    /**
     * VS-08: 空麦位（主麦 or 副麦）点击触发 onMicSlotClick 回调
     */
    @Test
    fun VS08_empty_slot_click_triggers_callback() {
        var clicked = false
        val slots = fullSlots()
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots, onMicSlotClick = { clicked = true })
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_empty_3").performClick()

        assertEquals("空麦位应触发 onClick 回调", true, clicked)
    }

    // ── VS-09: 点击正确 index ——————————————————————————————————————————————————

    /**
     * VS-09: 点击空副麦位，onMicSlotClick(index) 的 index 与 slot.index 一致
     */
    @Test
    fun VS09_click_guest_empty_slot_returns_correct_index() {
        var receivedIndex = -1
        val slots = fullSlots()
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots, onMicSlotClick = { receivedIndex = it })
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_empty_5").performClick()

        assertEquals("onMicSlotClick 应接收到 index=5", 5, receivedIndex)
    }

    // ── VS-18: 空列表 ——————————————————————————————————————————————————————————

    /**
     * VS-18: slots 列表为空时，MicSlotsGrid 不崩溃
     */
    @Test
    fun VS18_empty_slots_list_does_not_crash() {
        composeTestRule.setContent {
            MicSlotsGrid(slots = emptyList())
        }
        composeTestRule.waitForIdle()

        // mic_slots_grid 存在但无子麦位
        composeTestRule.onNodeWithTag("mic_slots_grid").assertExists()
    }

    // ── VS-19: 只有 1 个元素（主麦无副麦）——————————————————————————————————————

    /**
     * VS-19: slots 仅 1 个元素（只有主麦，无副麦），副麦 Grid 渲染 0 项不崩溃
     */
    @Test
    fun VS19_single_host_slot_no_guest_grid_does_not_crash() {
        val slots = listOf(occupiedSlot(0))
        composeTestRule.setContent {
            MicSlotsGrid(slots = slots)
        }
        composeTestRule.waitForIdle()

        // 主麦正常渲染
        composeTestRule.onNodeWithTag("mic_slot_occupied_0").assertIsDisplayed()
        // mic_slots_grid 存在（0 项副麦）
        composeTestRule.onNodeWithTag("mic_slots_grid").assertExists()
        // 没有副麦节点
        composeTestRule.onNodeWithTag("mic_slot_empty_1").assertDoesNotExist()
    }

    // ── VS-20: RTL 布局 ————————————————————————————————————————————————————————

    /**
     * VS-20: RTL 布局下，MicSlotsGrid 不崩溃，主麦居中显示
     */
    @Test
    fun VS20_rtl_layout_does_not_crash_host_slot_visible() {
        val slots = fullSlots()
        composeTestRule.setContent {
            CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Rtl) {
                MicSlotsGrid(slots = slots)
            }
        }
        composeTestRule.waitForIdle()

        // 主麦在 RTL 下仍可见
        composeTestRule.onNodeWithTag("mic_slot_empty_0").assertIsDisplayed()
        // 副麦在 RTL 下仍可见
        composeTestRule.onNodeWithTag("mic_slot_empty_1").assertIsDisplayed()
        // mic_slots_grid 在 RTL 下可见
        composeTestRule.onNodeWithTag("mic_slots_grid").assertIsDisplayed()
    }

    // ── WS 回归测试（VS-16 / VS-17）——主要在 RoomScreenBlackGoldTest 中覆盖，此处补充渲染层——

    /**
     * VS-16 渲染层验证: MicTaken 事件后 slot[0] 从空→有人，HostMicSlot 有人状态渲染正确
     * （ViewModel 逻辑在 RoomViewModelTest 验证，此处仅验证 UI 渲染层）
     */
    @Test
    fun VS16_host_slot_occupied_state_renders_correctly_after_mic_taken() {
        val occupiedSlots = listOf(occupiedSlot(0)) + List(8) { emptySlot(it + 1) }
        composeTestRule.setContent {
            MicSlotsGrid(slots = occupiedSlots)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_occupied_0").assertIsDisplayed()
    }

    /**
     * VS-17 渲染层验证: MicLeft 事件后 slot 从有人→空，对应位置空麦状态渲染正确
     */
    @Test
    fun VS17_slot_empty_state_renders_correctly_after_mic_left() {
        val slotsAfterLeave = List(9) { emptySlot(it) }
        composeTestRule.setContent {
            MicSlotsGrid(slots = slotsAfterLeave)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("mic_slot_empty_3").assertIsDisplayed()
    }
}
