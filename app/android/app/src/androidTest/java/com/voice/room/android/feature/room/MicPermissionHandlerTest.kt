package com.voice.room.android.feature.room

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — MicPermissionHandler (T-30012)
 *
 * 验收用例 MP-01～MP-07 及新增 MP-01B、MP-05B（Review R1 修复）。
 *
 * 通过向 MicPermissionHandler 注入 [MicPermissionHelper] 假实现进行隔离测试，
 * 避免对真实设备权限系统的依赖。
 *
 * CI 环境无真实设备，仅验证编译通过（compileDebugAndroidTestKotlin）。
 */
@RunWith(AndroidJUnit4::class)
class MicPermissionHandlerTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    // ── 辅助：创建假 MicPermissionHelper ─────────────────────────────────────

    private fun fakeGrantedHelper(launchCount: MutableList<Int> = mutableListOf()): MicPermissionHelper =
        object : MicPermissionHelper {
            override val isGranted = true
            override val shouldShowRationale = false
            override fun launchPermissionRequest() { launchCount.add(1) }
        }

    private fun fakeDeniedRationaleHelper(launchCount: MutableList<Int> = mutableListOf()): MicPermissionHelper =
        object : MicPermissionHelper {
            override val isGranted = false
            override val shouldShowRationale = true
            override fun launchPermissionRequest() { launchCount.add(1) }
        }

    private fun fakePermanentlyDeniedHelper(launchCount: MutableList<Int> = mutableListOf()): MicPermissionHelper =
        object : MicPermissionHelper {
            override val isGranted = false
            override val shouldShowRationale = false
            override fun launchPermissionRequest() { launchCount.add(1) }
        }

    // ── MP-01: 权限已授予 → 点击空麦位 → onPermissionGranted 调用，无对话框 ────

    /**
     * MP-01: 当权限已授予时，点击空麦位应直接调用 onPermissionGranted，不显示任何对话框。
     */
    @Test
    fun mp01_permissionGranted_clickSlot_callsGrantedCallback_noDialog() {
        val grantedCallbacks = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakeGrantedHelper(),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(0) },
                    modifier = Modifier.testTag("test_slot_0"),
                ) {
                    androidx.compose.material3.Text("点击麦位 0")
                }
            }
        }

        composeTestRule.onNodeWithTag("test_slot_0").performClick()

        // onPermissionGranted(0) 应被调用
        assertEquals("onPermissionGranted should be called with slotIndex=0", listOf(0), grantedCallbacks)
        // 无对话框
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertDoesNotExist()
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertDoesNotExist()
    }

    // ── MP-01B: 权限状态从 false 变为 true → LaunchedEffect 自动回调 ────────────

    /**
     * MP-01B: 点击麦位触发请求（isGranted=false），之后 isGranted 变为 true 时，
     * LaunchedEffect 应自动调用 onPermissionGranted，无需再次点击。
     */
    @Test
    fun mp01b_isGranted_changesTrue_afterRequest_callsGrantedCallback() {
        val grantedCallbacks = mutableListOf<Int>()
        val launchCount = mutableListOf<Int>()

        // 使用 Compose MutableState 包装 isGranted，使 Compose 能感知状态变化
        var isGrantedState by mutableStateOf(false)

        val mutableHelper = object : MicPermissionHelper {
            override val isGranted: Boolean get() = isGrantedState
            override val shouldShowRationale = false
            override fun launchPermissionRequest() { launchCount.add(1) }
        }

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = mutableHelper,
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(3) },
                    modifier = Modifier.testTag("test_slot_3"),
                ) {
                    androidx.compose.material3.Text("点击麦位 3")
                }
            }
        }

        // 首次点击：isGranted=false, permissionRequested=false → 触发权限请求
        composeTestRule.onNodeWithTag("test_slot_3").performClick()
        assertEquals("launchPermissionRequest should be called once", 1, launchCount.size)
        assertTrue("onPermissionGranted should NOT be called yet", grantedCallbacks.isEmpty())

        // 模拟系统授予权限
        isGrantedState = true
        composeTestRule.waitForIdle()

        // LaunchedEffect(helper.isGranted) 应触发 onPermissionGranted(3)
        assertTrue(
            "onPermissionGranted should be called after isGranted becomes true",
            grantedCallbacks.contains(3),
        )
    }

    // ── MP-02: shouldShowRationale=true → 点击麦位 → 理由对话框可见 ────────────

    /**
     * MP-02: 当 shouldShowRationale=true 时，点击麦位应显示理由对话框。
     */
    @Test
    fun mp02_shouldShowRationale_clickSlot_rationaleDialogVisible() {
        val grantedCallbacks = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakeDeniedRationaleHelper(),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(1) },
                    modifier = Modifier.testTag("test_slot_1"),
                ) {
                    androidx.compose.material3.Text("点击麦位 1")
                }
            }
        }

        composeTestRule.onNodeWithTag("test_slot_1").performClick()

        // 理由对话框应可见
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertIsDisplayed()
        // onPermissionGranted 不应被调用
        assertTrue("onPermissionGranted should NOT be called yet", grantedCallbacks.isEmpty())
    }

    // ── MP-03: 理由对话框确认 → 对话框消失 ─────────────────────────────────────

    /**
     * MP-03: 点击理由对话框的"允许"按钮后，对话框应消失并触发权限请求。
     */
    @Test
    fun mp03_rationaleDialog_confirm_dialogDismissed() {
        val launchCount = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = {},
                permissionHelperOverride = fakeDeniedRationaleHelper(launchCount),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(0) },
                    modifier = Modifier.testTag("test_slot"),
                ) {
                    androidx.compose.material3.Text("点击")
                }
            }
        }

        // 触发显示对话框
        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertIsDisplayed()

        // 点击确认
        composeTestRule.onNodeWithTag("rationale_confirm_button").performClick()

        // 对话框消失
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertDoesNotExist()
        // launchPermissionRequest 被调用
        assertTrue("launchPermissionRequest should be called after confirm", launchCount.isNotEmpty())
    }

    // ── MP-04: 理由对话框取消 → 对话框消失，onPermissionGranted 未调用 ───────────

    /**
     * MP-04: 点击理由对话框的"取消"按钮后，对话框消失且 onPermissionGranted 不被调用。
     */
    @Test
    fun mp04_rationaleDialog_dismiss_dialogDismissed_noCallback() {
        val grantedCallbacks = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakeDeniedRationaleHelper(),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(0) },
                    modifier = Modifier.testTag("test_slot"),
                ) {
                    androidx.compose.material3.Text("点击")
                }
            }
        }

        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertIsDisplayed()

        // 点击取消
        composeTestRule.onNodeWithTag("rationale_dismiss_button").performClick()

        // 对话框消失
        composeTestRule.onNodeWithTag("mic_permission_rationale_dialog").assertDoesNotExist()
        // onPermissionGranted 未被调用
        assertTrue("onPermissionGranted should NOT be called after dismiss", grantedCallbacks.isEmpty())
    }

    // ── MP-05: 首次请求（permissionRequested=false）→ 触发权限请求，不弹设置对话框 ──

    /**
     * MP-05（修复）: 当 shouldShowRationale=false 且 isGranted=false 且从未请求过权限时，
     * 点击麦位应触发 launchPermissionRequest()，而 **不** 弹出系统设置对话框。
     *
     * 根因修复：引入 permissionRequested 状态区分"首次请求"和"永久拒绝"。
     */
    @Test
    fun mp05_firstRequest_clickSlot_launchesRequest_noSettingsDialog() {
        val grantedCallbacks = mutableListOf<Int>()
        val launchCount = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakePermanentlyDeniedHelper(launchCount),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(2) },
                    modifier = Modifier.testTag("test_slot_2"),
                ) {
                    androidx.compose.material3.Text("点击麦位 2")
                }
            }
        }

        // 首次点击：permissionRequested=false → 触发权限请求
        composeTestRule.onNodeWithTag("test_slot_2").performClick()

        // 应触发 launchPermissionRequest，而非弹出设置对话框
        assertTrue("launchPermissionRequest should be called on first click", launchCount.isNotEmpty())
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertDoesNotExist()
        assertTrue("onPermissionGranted should NOT be called", grantedCallbacks.isEmpty())
    }

    // ── MP-05B: 已请求过且永久拒绝 → 弹出系统设置对话框 ─────────────────────────

    /**
     * MP-05B: 当 permissionRequested=true（已请求过），再次点击应弹出系统设置对话框。
     *
     * 通过点击两次同一麦位模拟：
     *   第一次点击 → launchPermissionRequest()（permissionRequested 变为 true）
     *   第二次点击 → 显示系统设置对话框
     */
    @Test
    fun mp05b_secondClick_permanentlyDenied_settingsDialogVisible() {
        val grantedCallbacks = mutableListOf<Int>()
        val launchCount = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakePermanentlyDeniedHelper(launchCount),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(2) },
                    modifier = Modifier.testTag("test_slot_2b"),
                ) {
                    androidx.compose.material3.Text("点击麦位 2")
                }
            }
        }

        // 第一次点击：触发权限请求，permissionRequested 变 true
        composeTestRule.onNodeWithTag("test_slot_2b").performClick()
        assertTrue("launchPermissionRequest should be called once", launchCount.size == 1)
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertDoesNotExist()

        // 第二次点击：permissionRequested=true → 弹出设置对话框
        composeTestRule.onNodeWithTag("test_slot_2b").performClick()

        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertIsDisplayed()
        assertTrue("onPermissionGranted should NOT be called", grantedCallbacks.isEmpty())
    }

    // ── MP-06: 设置对话框"去设置" → 对话框消失（Intent 已发出）──────────────────

    /**
     * MP-06: 点击设置对话框的"去设置"按钮后，对话框消失（Intent 跳转由平台处理）。
     *
     * 需要点击两次麦位才能触发设置对话框（第一次触发 launchPermissionRequest，
     * 第二次才因 permissionRequested=true 弹出设置对话框）。
     */
    @Test
    fun mp06_settingsDialog_confirm_dialogDismissed() {
        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = {},
                permissionHelperOverride = fakePermanentlyDeniedHelper(),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(0) },
                    modifier = Modifier.testTag("test_slot"),
                ) {
                    androidx.compose.material3.Text("点击")
                }
            }
        }

        // 两次点击才能打开设置对话框
        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertIsDisplayed()

        // 点击"去设置"
        composeTestRule.onNodeWithTag("settings_confirm_button").performClick()

        // 对话框消失（Intent 跳转已发出）
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertDoesNotExist()
    }

    // ── MP-07: 设置对话框取消 → onPermissionGranted 未调用 ──────────────────────

    /**
     * MP-07: 点击设置对话框的"取消"按钮后，对话框消失且 onPermissionGranted 不被调用。
     *
     * 需要点击两次麦位才能触发设置对话框。
     */
    @Test
    fun mp07_settingsDialog_dismiss_noCallback() {
        val grantedCallbacks = mutableListOf<Int>()

        composeTestRule.setContent {
            MicPermissionHandler(
                onPermissionGranted = { grantedCallbacks.add(it) },
                permissionHelperOverride = fakePermanentlyDeniedHelper(),
            ) { onMicSlotClick ->
                androidx.compose.material3.Button(
                    onClick = { onMicSlotClick(0) },
                    modifier = Modifier.testTag("test_slot"),
                ) {
                    androidx.compose.material3.Text("点击")
                }
            }
        }

        // 两次点击才能打开设置对话框
        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("test_slot").performClick()
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertIsDisplayed()

        // 点击取消
        composeTestRule.onNodeWithTag("settings_dismiss_button").performClick()

        // 对话框消失
        composeTestRule.onNodeWithTag("mic_permission_settings_dialog").assertDoesNotExist()
        // onPermissionGranted 未被调用
        assertTrue("onPermissionGranted should NOT be called after dismiss", grantedCallbacks.isEmpty())
    }
}
