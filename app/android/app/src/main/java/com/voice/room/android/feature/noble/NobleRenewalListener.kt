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
 * NobleRenewalListener — 贵族 WS 信号处理器 (T-30075 + P0-2)
 *
 * 监听并处理 6 个贵族 WS 信号：
 * - NobleRenewFailed / NobleExpired → AlertDialog 提示
 * - NobleRenewSuccess → onNobleChanged 回调
 * - NobleChanged / NobleEntered / NobleEntranceGlobal → 投递至 UI
 * 24h 内同事件不重弹。
 */
@Composable
fun NobleRenewalListener(
    wsClient: IWebSocketClient,
    onNavigateToNobleCenter: () -> Unit,
    onNobleChanged: (() -> Unit)? = null,
    onNobleEntered: ((NobleEntrance) -> Unit)? = null,
) {
    var showFailedDialog by remember { mutableStateOf(false) }
    var showExpiredDialog by remember { mutableStateOf(false) }
    var showRenewSuccess by remember { mutableStateOf(false) }
    var failedReason by remember { mutableStateOf("") }
    val gson = remember { Gson() }

    LaunchedEffect(wsClient) {
        withContext(Dispatchers.IO) {
            wsClient.state.collect { state ->
                when (state) {
                    is WebSocketState.Message -> {
                        val text = state.text
                        when {
                            // ── NobleRenewFailed ──
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
                            // ── NobleExpired ──
                            text.contains("NobleExpired") -> {
                                val now = System.currentTimeMillis()
                                if (now - lastDismissedExpiredMs > 86_400_000) {
                                    showExpiredDialog = true
                                }
                            }
                            // ── NobleRenewSuccess ── (P0-2)
                            text.contains("NobleRenewSuccess") -> {
                                showRenewSuccess = true
                                onNobleChanged?.invoke()
                            }
                            // ── NobleChanged ── (P0-2: purchase/upgrade/grant/revoke)
                            text.contains("NobleChanged") -> {
                                onNobleChanged?.invoke()
                            }
                            // ── NobleEntered ── (P0-2: room-level Lv3+ entrance)
                            text.contains("NobleEntered") -> {
                                try {
                                    val entrance = gson.fromJson(text, NobleEntrance::class.java)
                                    onNobleEntered?.invoke(entrance)
                                } catch (_: Exception) {}
                            }
                            // ── NobleEntranceGlobal ── (P0-2: global Lv5+ marquee)
                            text.contains("NobleEntranceGlobal") -> {
                                try {
                                    val entrance = gson.fromJson(text, NobleEntrance::class.java)
                                    onNobleEntered?.invoke(entrance)
                                } catch (_: Exception) {}
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

    if (showRenewSuccess) {
        AlertDialog(
            onDismissRequest = { showRenewSuccess = false },
            title = { Text("Renewal Successful") },
            text = { Text("Your noble status has been renewed.") },
            confirmButton = {
                TextButton(onClick = { showRenewSuccess = false }) { Text("OK") }
            },
            modifier = Modifier.testTag("noble_renew_success_dialog")
        )
    }
}

data class RenewFailedEvent(
    @SerializedName("reason") val reason: String?
)
