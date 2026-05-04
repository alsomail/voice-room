package com.voice.room.android.feature.room

import android.content.Intent
import android.net.Uri
import android.provider.Settings
import android.widget.Toast
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.PermissionState
import com.google.accompanist.permissions.isGranted
import com.google.accompanist.permissions.rememberPermissionState
import com.google.accompanist.permissions.shouldShowRationale

// ─────────────────────────────────────────────
// 内部抽象：隔离 Accompanist 实验性 API
// ─────────────────────────────────────────────

/**
 * 麦克风权限状态的抽象接口。
 *
 * 将 Accompanist 的 [PermissionState]（实验性 API）封装在此接口后面，
 * 使 [MicPermissionHandler] 的公共签名不暴露任何实验性类型，
 * 从而调用方无需标注 [@OptIn]。
 *
 * 测试注入假实现；生产由 [RealMicPermissionHelper] 封装真实 PermissionState。
 */
interface MicPermissionHelper {
    val isGranted: Boolean
    val shouldShowRationale: Boolean
    fun launchPermissionRequest()
}

/**
 * 生产环境实现：委托给 Accompanist [PermissionState]。
 */
@OptIn(ExperimentalPermissionsApi::class)
class RealMicPermissionHelper(private val state: PermissionState) : MicPermissionHelper {
    override val isGranted: Boolean get() = state.status.isGranted
    override val shouldShowRationale: Boolean get() = state.status.shouldShowRationale
    override fun launchPermissionRequest() = state.launchPermissionRequest()
}

// ─────────────────────────────────────────────
// MicPermissionHandler
// ─────────────────────────────────────────────

/**
 * 麦克风权限守卫 Composable (T-30012)
 *
 * 包裹 [content]，拦截 onMicSlotClick 并根据权限状态：
 * - GRANTED            → 直接透传 [onPermissionGranted]
 * - DENIED (rationale) → 显示 [MicPermissionRationaleDialog]
 * - DENIED (permanent) → 显示 [MicPermissionSettingsDialog]
 *
 * @param onPermissionGranted      权限授予后的回调，参数为麦位 index
 * @param permissionHelperOverride 仅供测试注入假实现，生产传 null（使用真实 Accompanist 状态）
 * @param content                  子 Composable，接收 onMicSlotClick 回调
 */
@OptIn(ExperimentalPermissionsApi::class)
@Composable
fun MicPermissionHandler(
    onPermissionGranted: (slotIndex: Int) -> Unit,
    permissionHelperOverride: MicPermissionHelper? = null,
    content: @Composable (onMicSlotClick: (Int) -> Unit) -> Unit,
) {
    // LOW: LocalContext.current 提升至 composable 顶部，不在条件块内读取
    val context = LocalContext.current

    val helper: MicPermissionHelper = permissionHelperOverride
        ?: RealMicPermissionHelper(rememberPermissionState(android.Manifest.permission.RECORD_AUDIO))

    var pendingSlotIndex by remember { mutableStateOf<Int?>(null) }
    var showRationaleDialog by remember { mutableStateOf(false) }
    var showSettingsDialog by remember { mutableStateOf(false) }
    // HIGH: 引入 permissionRequested 区分"首次请求"与"永久拒绝"
    var permissionRequested by remember { mutableStateOf(false) }
    // BUG-MIC-PERMISSION-TOAST Round 6：追踪「正在等待系统权限弹窗结果」，
    // 系统弹窗关闭后若仍未授予，需要给用户一个反馈，避免静默无响应。
    var awaitingPermissionResult by remember { mutableStateOf(false) }

    // 当权限状态变为授予且有 pending index 时，透传回调（处理系统权限弹窗返回后的情况）
    LaunchedEffect(helper.isGranted) {
        if (helper.isGranted) {
            awaitingPermissionResult = false
            pendingSlotIndex?.let { idx ->
                onPermissionGranted(idx)
                pendingSlotIndex = null
            }
        }
    }

    // BUG-MIC-PERMISSION-TOAST Round 6：当用户从系统权限弹窗返回但未授予时，
    // 通过 shouldShowRationale 的变化感知到弹窗已关闭，弹 Toast 提示，避免无反馈。
    LaunchedEffect(awaitingPermissionResult, helper.shouldShowRationale, helper.isGranted) {
        if (awaitingPermissionResult && !helper.isGranted && helper.shouldShowRationale) {
            awaitingPermissionResult = false
            Toast.makeText(
                context,
                "麦克风权限被拒绝，请允许后再上麦",
                Toast.LENGTH_SHORT,
            ).show()
        }
    }

    val onMicSlotClick: (Int) -> Unit = { slotIndex ->
        when {
            helper.isGranted -> {
                onPermissionGranted(slotIndex)
            }
            helper.shouldShowRationale -> {
                pendingSlotIndex = slotIndex
                showRationaleDialog = true
            }
            else -> {
                pendingSlotIndex = slotIndex
                if (permissionRequested) {
                    // 已请求过且仍未授予 → 永久拒绝，引导去设置
                    showSettingsDialog = true
                } else {
                    // 首次请求：直接触发系统权限弹窗
                    permissionRequested = true
                    awaitingPermissionResult = true
                    helper.launchPermissionRequest()
                }
            }
        }
    }

    content(onMicSlotClick)

    if (showRationaleDialog) {
        MicPermissionRationaleDialog(
            onConfirm = {
                showRationaleDialog = false
                awaitingPermissionResult = true
                helper.launchPermissionRequest()
            },
            onDismiss = {
                showRationaleDialog = false
                pendingSlotIndex = null
                // BUG-MIC-PERMISSION-TOAST Round 6：用户主动关闭 rationale 也给一次反馈
                Toast.makeText(
                    context,
                    "已取消，可在系统设置中开启麦克风权限",
                    Toast.LENGTH_SHORT,
                ).show()
            },
        )
    }

    if (showSettingsDialog) {
        MicPermissionSettingsDialog(
            onConfirm = {
                showSettingsDialog = false
                pendingSlotIndex = null
                val intent = Intent(
                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                    Uri.fromParts("package", context.packageName, null),
                ).apply { addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) }
                context.startActivity(intent)
            },
            onDismiss = {
                showSettingsDialog = false
                pendingSlotIndex = null
            },
        )
    }
}

// ─────────────────────────────────────────────
// 理由对话框
// ─────────────────────────────────────────────

/**
 * 麦克风权限理由对话框（首次拒绝场景）。
 *
 * testTags:
 * - 对话框容器：`mic_permission_rationale_dialog`
 * - 确认按钮：`rationale_confirm_button`
 * - 取消按钮：`rationale_dismiss_button`
 */
@Composable
fun MicPermissionRationaleDialog(
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = Modifier.testTag("mic_permission_rationale_dialog"),
        title = { Text("需要麦克风权限") },
        text = { Text("语聊房需要麦克风权限才能让你上麦发言，请允许此权限。") },
        confirmButton = {
            TextButton(
                onClick = onConfirm,
                modifier = Modifier.testTag("rationale_confirm_button"),
            ) { Text("允许") }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.testTag("rationale_dismiss_button"),
            ) { Text("取消") }
        },
    )
}

// ─────────────────────────────────────────────
// 系统设置对话框
// ─────────────────────────────────────────────

/**
 * 麦克风权限设置对话框（永久拒绝场景）。
 *
 * testTags:
 * - 对话框容器：`mic_permission_settings_dialog`
 * - 确认按钮：`settings_confirm_button`
 * - 取消按钮：`settings_dismiss_button`
 */
@Composable
fun MicPermissionSettingsDialog(
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = Modifier.testTag("mic_permission_settings_dialog"),
        title = { Text("麦克风权限被拒绝") },
        text = { Text("麦克风权限已被永久拒绝，请前往系统设置手动开启。") },
        confirmButton = {
            TextButton(
                onClick = onConfirm,
                modifier = Modifier.testTag("settings_confirm_button"),
            ) { Text("去设置") }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.testTag("settings_dismiss_button"),
            ) { Text("取消") }
        },
    )
}
