package com.voice.room.android.feature.room

import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — RoomBottomBar (T-30026)
 *
 * BB-01: RoomBottomBar 容器 ("room_bottom_bar") 可见，4 个图标按钮均可定位
 * BB-02: isOnMic=false → btn_mic_toggle 处于 disabled 状态
 * BB-03: isOnMic=true, isMicMuted=false → btn_mic_toggle 可见且 enabled（绿色 Mic 图标）
 * BB-04: isOnMic=true, isMicMuted=true  → btn_mic_toggle 可见且 enabled（红色 MicOff 图标）
 * BB-05: 点击 btn_gift → Toast "礼物功能敬请期待" 出现（或按钮可见且可点击）
 * BB-06: 点击 btn_emoji → Toast "表情功能敬请期待" 出现（或按钮可见且可点击）
 * BB-07: 点击 btn_exit_room → exit_room_dialog 弹出可见
 * BB-08: AlertDialog 确认 → onLeaveRoom 回调被调用一次
 * BB-09: AlertDialog 取消 → 弹窗消失，onLeaveRoom 不被调用
 * BB-10（回归）: chat_input_field / chat_send_button testTag 仍然可定位
 */
@RunWith(AndroidJUnit4::class)
class RoomBottomBarTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun launchBottomBar(
        isOnMic: Boolean = false,
        isMicMuted: Boolean = false,
        onMicToggle: () -> Unit = {},
        onLeaveRoom: () -> Unit = {},
        onSendMessage: (String) -> Unit = {},
    ) {
        composeTestRule.setContent {
            RoomBottomBar(
                inputText = "",
                onInputTextChange = {},
                isSending = false,
                onSendMessage = onSendMessage,
                isOnMic = isOnMic,
                isMicMuted = isMicMuted,
                onMicToggle = onMicToggle,
                onLeaveRoom = onLeaveRoom,
            )
        }
        composeTestRule.waitForIdle()
    }

    // ─── BB-01: 容器及 4 个图标按钮可见 ──────────────────────────────────────

    @Test
    fun BB01_bottomBarVisibleWithAllFourIconButtons() {
        launchBottomBar()

        composeTestRule.onNodeWithTag("room_bottom_bar").assertIsDisplayed()
        composeTestRule.onNodeWithTag("btn_mic_toggle").assertExists()
        composeTestRule.onNodeWithTag("btn_gift").assertExists()
        composeTestRule.onNodeWithTag("btn_emoji").assertExists()
        composeTestRule.onNodeWithTag("btn_exit_room").assertExists()
    }

    // ─── BB-02: isOnMic=false → btn_mic_toggle disabled ─────────────────────

    @Test
    fun BB02_micButton_isOnMicFalse_isDisabled() {
        launchBottomBar(isOnMic = false)

        composeTestRule.onNodeWithTag("btn_mic_toggle").assertIsNotEnabled()
    }

    // ─── BB-03: isOnMic=true, isMicMuted=false → btn_mic_toggle enabled ──────

    @Test
    fun BB03_micButton_isOnMicTrue_notMuted_isEnabled() {
        launchBottomBar(isOnMic = true, isMicMuted = false)

        composeTestRule.onNodeWithTag("btn_mic_toggle").assertIsEnabled()
        composeTestRule.onNodeWithTag("btn_mic_toggle").assertIsDisplayed()
    }

    // ─── BB-04: isOnMic=true, isMicMuted=true → btn_mic_toggle enabled ───────

    @Test
    fun BB04_micButton_isOnMicTrue_muted_isEnabled() {
        launchBottomBar(isOnMic = true, isMicMuted = true)

        composeTestRule.onNodeWithTag("btn_mic_toggle").assertIsEnabled()
        composeTestRule.onNodeWithTag("btn_mic_toggle").assertIsDisplayed()
    }

    // ─── BB-05: 点击礼物按钮 → 按钮可点击，Toast 展示（UI 层验证按钮可见并可点击）

    @Test
    fun BB05_giftButton_isClickable() {
        launchBottomBar()

        // btn_gift 可见且可以点击（内部会弹 Toast，UI 测试验证按钮存在且不崩溃）
        composeTestRule.onNodeWithTag("btn_gift").assertIsDisplayed()
        composeTestRule.onNodeWithTag("btn_gift").performClick()
        composeTestRule.waitForIdle()
        // 点击后不崩溃，按钮仍然可见
        composeTestRule.onNodeWithTag("btn_gift").assertIsDisplayed()
    }

    // ─── BB-06: 点击表情按钮 → 按钮可点击，Toast 展示 ──────────────────────

    @Test
    fun BB06_emojiButton_isClickable() {
        launchBottomBar()

        composeTestRule.onNodeWithTag("btn_emoji").assertIsDisplayed()
        composeTestRule.onNodeWithTag("btn_emoji").performClick()
        composeTestRule.waitForIdle()
        // 点击后不崩溃，按钮仍然可见
        composeTestRule.onNodeWithTag("btn_emoji").assertIsDisplayed()
    }

    // ─── BB-07: 点击退出按钮 → exit_room_dialog 弹出 ────────────────────────

    @Test
    fun BB07_exitButton_showsAlertDialog() {
        launchBottomBar()

        composeTestRule.onNodeWithTag("btn_exit_room").performClick()
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("exit_room_dialog").assertIsDisplayed()
    }

    // ─── BB-08: AlertDialog 确认 → onLeaveRoom 被调用 ───────────────────────

    @Test
    fun BB08_exitDialog_confirmButton_callsOnLeaveRoom() {
        var leaveRoomCalled = 0
        launchBottomBar(onLeaveRoom = { leaveRoomCalled++ })

        // 打开对话框
        composeTestRule.onNodeWithTag("btn_exit_room").performClick()
        composeTestRule.waitForIdle()

        // 点确认
        composeTestRule.onNodeWithText("确认").performClick()
        composeTestRule.waitForIdle()

        assertEquals("onLeaveRoom should be called exactly once", 1, leaveRoomCalled)
        // 对话框应消失
        composeTestRule.onNodeWithTag("exit_room_dialog").assertDoesNotExist()
    }

    // ─── BB-09: AlertDialog 取消 → 弹窗消失，onLeaveRoom 不调用 ────────────

    @Test
    fun BB09_exitDialog_cancelButton_dismissesDialogWithoutCallingOnLeaveRoom() {
        var leaveRoomCalled = 0
        launchBottomBar(onLeaveRoom = { leaveRoomCalled++ })

        // 打开对话框
        composeTestRule.onNodeWithTag("btn_exit_room").performClick()
        composeTestRule.waitForIdle()

        // 点取消
        composeTestRule.onNodeWithText("取消").performClick()
        composeTestRule.waitForIdle()

        assertEquals("onLeaveRoom should NOT be called on cancel", 0, leaveRoomCalled)
        composeTestRule.onNodeWithTag("exit_room_dialog").assertDoesNotExist()
    }

    // ─── BB-10（回归）: chat_input_field / chat_send_button 仍然可定位 ───────

    @Test
    fun BB10_chatInputFieldAndSendButtonStillReachable() {
        launchBottomBar()

        composeTestRule.onNodeWithTag("chat_input_field").assertIsDisplayed()
        composeTestRule.onNodeWithTag("chat_send_button").assertExists()
    }
}
