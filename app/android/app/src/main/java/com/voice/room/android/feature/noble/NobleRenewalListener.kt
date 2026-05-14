package com.voice.room.android.feature.noble

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.runtime.*
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Dedup: last dismissed timestamp */
private var lastDismissedFailedMs = 0L
private var lastDismissedExpiredMs = 0L

/**
 * NobleRenewalListener — 续费/过期/失败提醒 (T-30075)
 *
 * 监听 WS NobleRenewFailed / NobleExpired 事件，弹出 AlertDialog 提示。
 * 24h 内同事件不重弹。
 */
@Composable
fun NobleRenewalListener(
    wsClient: IWebSocketClient,
    onNavigateToNobleCenter: () -> Unit
) {
    var showFailedDialog by remember { mutableStateOf(false) }
    var showExpiredDialog by remember { mutableStateOf(false) }
    var failedReason by remember { mutableStateOf("") }
    val gson = remember { Gson() }

    // Listen to WS state for NobleRenewFailed/NobleExpired text frames
    LaunchedEffect(wsClient) {
        withContext(Dispatchers.IO) {
            wsClient.state.collect { state ->
                when (state) {
                    is WebSocketState.Message -> {
                        val text = state.text
                        when {
                            text.contains("NobleRenewFailed") -> {
                                val now = System.currentTimeMillis()
                                if (now - lastDismissedFailedMs > 86_400_000) {
                                    try {
                                        val event = gson.fromJson(text, RenewFailedEvent::class.java)
                                        failedReason = event.reason ?: "Insufficient balance"
                                    } catch (_: Exception) {
                                        failedReason = "Auto-renewal failed"
                                    }
                                    showFailedDialog = true
                                }
                            }
                            text.contains("NobleExpired") -> {
                                val now = System.currentTimeMillis()
                                if (now - lastDismissedExpiredMs > 86_400_000) {
                                    showExpiredDialog = true
                                }
                            }
                        }
                    }
                    else -> {}
                }
            }
        }
    }

    if (showFailedDialog) {
        AlertDialog(
            onDismissRequest = {
                showFailedDialog = false
                lastDismissedFailedMs = System.currentTimeMillis()
            },
            title = { Text("Renewal Failed") },
            text = { Text(failedReason.ifEmpty { "Auto-renewal failed. Insufficient balance." }) },
            confirmButton = {
                TextButton(onClick = {
                    showFailedDialog = false
                    lastDismissedFailedMs = System.currentTimeMillis()
                    onNavigateToNobleCenter()
                }) { Text("Recharge Now") }
            },
            dismissButton = {
                TextButton(onClick = {
                    showFailedDialog = false
                    lastDismissedFailedMs = System.currentTimeMillis()
                }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_renew_failed_dialog")
        )
    }

    if (showExpiredDialog) {
        AlertDialog(
            onDismissRequest = {
                showExpiredDialog = false
                lastDismissedExpiredMs = System.currentTimeMillis()
            },
            title = { Text("Noble Expired") },
            text = { Text("Your noble status has expired. Renew now?") },
            confirmButton = {
                TextButton(onClick = {
                    showExpiredDialog = false
                    lastDismissedExpiredMs = System.currentTimeMillis()
                    onNavigateToNobleCenter()
                }) { Text("Renew") }
            },
            dismissButton = {
                TextButton(onClick = {
                    showExpiredDialog = false
                    lastDismissedExpiredMs = System.currentTimeMillis()
                }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_expired_dialog")
        )
    }
}

data class RenewFailedEvent(
    @SerializedName("reason") val reason: String?
)
