package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

/**
 * 聊天输入栏组件 (T-30015)
 *
 * 无状态 Composable：状态完全由调用方控制，便于 T-30016 将 [inputText] 提升至 ViewModel。
 *
 * 布局：`Row`（[TextField] + [Spacer] + [IconButton]）
 * - 输入框：`testTag("chat_input_field")`，`weight(1f)` 填充剩余宽度
 * - 发送按钮：`testTag("chat_send_button")`
 *   - `enabled = inputText.isNotBlank() && !isSending`
 *   - 点击 / IME Send 动作均触发 [onSendMessage]（仅在 canSend 时）
 *
 * @param inputText         当前输入框文字（由调用方持有）
 * @param onInputTextChange 用户输入变化回调
 * @param onSendMessage     发送回调，参数为当前 [inputText]（调用方负责发送后清空）
 * @param modifier          可选 Modifier
 * @param isSending         发送中标志（预留 T-30016：发送中禁用按钮）
 */
@Composable
fun ChatInputBar(
    inputText: String,
    onInputTextChange: (String) -> Unit,
    onSendMessage: (String) -> Unit,
    modifier: Modifier = Modifier,
    isSending: Boolean = false,
) {
    val canSend = inputText.isNotBlank() && !isSending

    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextField(
            value = inputText,
            onValueChange = onInputTextChange,
            modifier = Modifier
                .weight(1f)
                .testTag("chat_input_field"),
            placeholder = { Text("说点什么...") },
            keyboardOptions = KeyboardOptions(
                imeAction = ImeAction.Send,
            ),
            keyboardActions = KeyboardActions(
                onSend = { if (canSend) onSendMessage(inputText) },
            ),
            singleLine = true,
        )

        Spacer(modifier = Modifier.width(8.dp))

        IconButton(
            onClick = { onSendMessage(inputText) },
            enabled = canSend,
            modifier = Modifier.testTag("chat_send_button"),
        ) {
            Icon(
                imageVector = Icons.AutoMirrored.Filled.Send,
                contentDescription = "发送",
            )
        }
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "ChatInputBar — 空输入")
@Composable
private fun ChatInputBarEmptyPreview() {
    ChatInputBar(
        inputText = "",
        onInputTextChange = {},
        onSendMessage = {},
    )
}

@Preview(showBackground = true, name = "ChatInputBar — 有内容")
@Composable
private fun ChatInputBarWithTextPreview() {
    ChatInputBar(
        inputText = "你好，世界！",
        onInputTextChange = {},
        onSendMessage = {},
    )
}

@Preview(showBackground = true, name = "ChatInputBar — 发送中")
@Composable
private fun ChatInputBarSendingPreview() {
    ChatInputBar(
        inputText = "发送中...",
        onInputTextChange = {},
        onSendMessage = {},
        isSending = true,
    )
}
