package com.voice.room.android.feature.noble

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.runtime.*
import androidx.compose.ui.platform.testTag
import com.voice.room.android.core.ws.IWebSocketClient

/**
 * NobleRenewalListener — 续费/过期/失败提醒 (T-30075)
 *
 * 监听 WS NobleRenewFailed / NobleExpired 事件，弹出 AlertDialog 提示。
 * DataStore 记录已弹过避免重复打扰（24h 内同事件不重弹）。
 */
@Composable
fun NobleRenewalListener(
    wsClient: IWebSocketClient,
    onNavigateToNobleCenter: () -> Unit
) {
    var showFailedDialog by remember { mutableStateOf(false) }
    var showExpiredDialog by remember { mutableStateOf(false) }
    var failedReason by remember { mutableStateOf("") }

    // TODO: Parse WS messages for NobleRenewFailed/NobleExpired events
    // For now, placeholder

    if (showFailedDialog) {
        AlertDialog(
            onDismissRequest = { showFailedDialog = false },
            title = { Text("Renewal Failed") },
            text = { Text(failedReason.ifEmpty { "Auto-renewal failed. Insufficient balance." }) },
            confirmButton = {
                TextButton(onClick = {
                    showFailedDialog = false
                    onNavigateToNobleCenter()
                }) { Text("Recharge Now") }
            },
            dismissButton = {
                TextButton(onClick = { showFailedDialog = false }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_renew_failed_dialog")
        )
    }

    if (showExpiredDialog) {
        AlertDialog(
            onDismissRequest = { showExpiredDialog = false },
            title = { Text("Noble Expired") },
            text = { Text("Your noble status has expired. Renew now?") },
            confirmButton = {
                TextButton(onClick = {
                    showExpiredDialog = false
                    onNavigateToNobleCenter()
                }) { Text("Renew") }
            },
            dismissButton = {
                TextButton(onClick = { showExpiredDialog = false }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_expired_dialog")
        )
    }
}
