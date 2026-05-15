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
import com.google.gson.JsonParser
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private var lastDismissedFailedMs = 0L
private var lastDismissedExpiredMs = 0L

/**
 * NobleRenewalListener — 贵族全量 WS 信号处理 (T-30075)
 *
 * 从 WS envelope 中提取 payload 子对象并解析，覆盖 6 个信号：
 * NobleRenewFailed / NobleExpired / NobleRenewSuccess / NobleChanged / NobleEntered / NobleEntranceGlobal
 */
@Composable
fun NobleRenewalListener(
    wsClient: IWebSocketClient,
    onNavigateToNobleCenter: () -> Unit,
) {
    var showFailedDialog by remember { mutableStateOf(false) }
    var showExpiredDialog by remember { mutableStateOf(false) }
    var showRenewSuccess by remember { mutableStateOf(false) }
    var failedReason by remember { mutableStateOf("") }
    val gson = remember { Gson() }

    LaunchedEffect(wsClient) {
        withContext(Dispatchers.IO) {
            wsClient.state.collect { state ->
                if (state !is WebSocketState.Message) return@collect
                val text = state.text
                val type = try {
                    JsonParser.parseString(text).asJsonObject.get("type")?.asString ?: return@collect
                } catch (_: Exception) { return@collect }
                // Extract payload sub-object for accurate deserialization
                val payloadJson = try {
                    JsonParser.parseString(text).asJsonObject.get("payload")?.toString() ?: text
                } catch (_: Exception) { text }

                when (type) {
                    "NobleRenewFailed" -> {
                        val now = System.currentTimeMillis()
                        if (now - lastDismissedFailedMs > 86_400_000) {
                            try {
                                val event = gson.fromJson(payloadJson, RenewFailedPayload::class.java)
                                failedReason = event.reason ?: "Insufficient balance"
                            } catch (_: Exception) { failedReason = "Auto-renewal failed" }
                            showFailedDialog = true
                        }
                    }
                    "NobleExpired" -> {
                        val now = System.currentTimeMillis()
                        if (now - lastDismissedExpiredMs > 86_400_000) { showExpiredDialog = true }
                    }
                    "NobleRenewSuccess" -> { showRenewSuccess = true }
                    "NobleChanged" -> { /* Handled by caller via recomposition */ }
                    "NobleEntered", "NobleEntranceGlobal" -> { /* Handled by NobleEntrancePlayer */ }
                }
            }
        }
    }

    if (showFailedDialog) {
        AlertDialog(
            onDismissRequest = { showFailedDialog = false; lastDismissedFailedMs = System.currentTimeMillis() },
            title = { Text("Renewal Failed") },
            text = { Text(failedReason.ifEmpty { "Auto-renewal failed. Insufficient balance." }) },
            confirmButton = {
                TextButton(onClick = {
                    showFailedDialog = false; lastDismissedFailedMs = System.currentTimeMillis()
                    onNavigateToNobleCenter()
                }) { Text("Recharge Now") }
            },
            dismissButton = {
                TextButton(onClick = { showFailedDialog = false; lastDismissedFailedMs = System.currentTimeMillis() }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_renew_failed_dialog")
        )
    }

    if (showExpiredDialog) {
        AlertDialog(
            onDismissRequest = { showExpiredDialog = false; lastDismissedExpiredMs = System.currentTimeMillis() },
            title = { Text("Noble Expired") },
            text = { Text("Your noble status has expired. Renew now?") },
            confirmButton = {
                TextButton(onClick = {
                    showExpiredDialog = false; lastDismissedExpiredMs = System.currentTimeMillis()
                    onNavigateToNobleCenter()
                }) { Text("Renew") }
            },
            dismissButton = {
                TextButton(onClick = { showExpiredDialog = false; lastDismissedExpiredMs = System.currentTimeMillis() }) { Text("Dismiss") }
            },
            modifier = Modifier.testTag("noble_expired_dialog")
        )
    }

    if (showRenewSuccess) {
        AlertDialog(
            onDismissRequest = { showRenewSuccess = false },
            title = { Text("Renewal Successful") },
            text = { Text("Your noble status has been renewed.") },
            confirmButton = { TextButton(onClick = { showRenewSuccess = false }) { Text("OK") } },
            modifier = Modifier.testTag("noble_renew_success_dialog")
        )
    }
}

data class RenewFailedPayload(
    @SerializedName("reason") val reason: String?
)
