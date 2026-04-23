package com.voice.room.android.feature.room.governance

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties

/**
 * 踢人原因选择弹窗（T-30041）
 *
 * 功能：
 * - 展示 4 个预设踢出原因（Harassment / Spam / Abuse / Other）
 * - 选择"其他"时显示 [OutlinedTextField]（最多 100 字符）
 * - 确认按钮遵循 [KickDialogState.canSubmit] 规则（灰化时不可点击）
 * - 不允许点击外部关闭（[DialogProperties.dismissOnClickOutside] = false）
 *
 * @param targetUserId 目标用户 ID，传给 [onConfirm]
 * @param onConfirm    用户点击确认后回调，参数为 (targetUserId, reasonText)
 * @param onDismiss    用户点击取消后回调
 */
@Composable
fun KickReasonDialog(
    targetUserId: String,
    onConfirm: (targetUserId: String, reason: String) -> Unit,
    onDismiss: () -> Unit,
) {
    var state by rememberSaveable {
        mutableStateOf(KickDialogState())
    }

    AlertDialog(
        onDismissRequest = { /* 禁止外部 dismiss */ },
        properties = DialogProperties(
            dismissOnClickOutside = false,
            dismissOnBackPress = false,
        ),
        title = {
            Text(text = "踢出原因")
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .semantics { testTag = "kick_reason_dialog" }
            ) {
                KickReason.values().forEachIndexed { index, reason ->
                    val label = when (reason) {
                        KickReason.Harassment -> "骚扰"
                        KickReason.Spam       -> "刷屏"
                        KickReason.Abuse      -> "辱骂"
                        KickReason.Other      -> "其他"
                    }
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .semantics { testTag = "kick_reason_$index" }
                    ) {
                        RadioButton(
                            selected = state.selected == reason,
                            onClick = { state = state.copy(selected = reason, customText = "") }
                        )
                        Text(text = label)
                    }
                }

                // "其他"选项展开自定义输入框
                if (state.selected == KickReason.Other) {
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = state.customText,
                        onValueChange = { text ->
                            if (text.length <= 100) {
                                state = state.copy(customText = text)
                            }
                        },
                        placeholder = { Text("请输入原因（最多100字）") },
                        singleLine = false,
                        maxLines = 4,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 4.dp)
                            .semantics { testTag = "kick_custom_input" },
                    )
                }
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.semantics { testTag = "btn_cancel_kick" }
            ) {
                Text("取消")
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    if (!state.canSubmit) return@Button
                    state = state.copy(submitting = true)
                    val reasonText = when (state.selected) {
                        KickReason.Other -> state.customText.trim()
                        else             -> state.selected.key
                    }
                    onConfirm(targetUserId, reasonText)
                },
                enabled = state.canSubmit,
                colors = ButtonDefaults.buttonColors(
                    disabledContainerColor = Color.Gray.copy(alpha = 0.4f),
                ),
                modifier = Modifier.semantics { testTag = "btn_confirm_kick" },
            ) {
                Text("确认")
            }
        },
    )
}
