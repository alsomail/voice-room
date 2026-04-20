package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

/**
 * 底部输入栏 (T-30009)
 *
 * 布局：`Row`（输入框 + 发送按钮）
 * - 输入框：`testTag("message_input")`，`weight(1f)` 填充剩余宽度
 * - 发送按钮：`testTag("send_button")`，消息为空时 `enabled = false`
 * - 点击发送后清空输入框
 *
 * @param onSend   点击发送按钮的回调，参数为消息文本
 * @param modifier 可选 Modifier
 */
@Composable
fun BottomInputBar(
    onSend: (String) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    var inputText by remember { mutableStateOf("") }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        OutlinedTextField(
            value = inputText,
            onValueChange = { inputText = it },
            placeholder = {
                Text(
                    text = "说点什么…",
                    style = MaterialTheme.typography.bodyMedium,
                )
            },
            singleLine = true,
            modifier = Modifier
                .weight(1f)
                .testTag("message_input"),
        )

        Button(
            onClick = {
                val message = inputText.trim()
                if (message.isNotEmpty()) {
                    onSend(message)
                    inputText = ""
                }
            },
            enabled = inputText.trim().isNotEmpty(),
            modifier = Modifier
                .padding(start = 8.dp)
                .testTag("send_button"),
        ) {
            Text("发送")
        }
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "BottomInputBar — 预览")
@Composable
private fun BottomInputBarPreview() {
    BottomInputBar(onSend = {})
}
