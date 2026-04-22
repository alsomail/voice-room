package com.voice.room.android.feature.room.create.components

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

/**
 * 6 位分格密码输入组件（T-30036）
 *
 * 每位数字显示在独立的方格中，仅允许输入 0-9 数字，最多 6 位。
 *
 * @param value     当前密码字符串（0-6 位）
 * @param onValueChange 密码变化回调（只传递纯数字，最多 6 位）
 * @param modifier  可选 Modifier
 * @param enabled   是否可交互
 */
@Composable
fun PasswordInputRow(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true
) {
    val cellCount = 6

    Box(
        modifier = modifier.fillMaxWidth(),
        contentAlignment = Alignment.Center
    ) {
        // 隐藏的真实输入框，捕捉键盘输入
        BasicTextField(
            value = value,
            onValueChange = { newVal ->
                // 只接受纯数字，最多 6 位
                val filtered = newVal.filter { it.isDigit() }.take(cellCount)
                onValueChange(filtered)
            },
            enabled = enabled,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
            modifier = Modifier.testTag("password_input_hidden"),
            decorationBox = {
                // 用 6 个方格展示每一位
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally)
                ) {
                    repeat(cellCount) { index ->
                        val char = value.getOrNull(index)
                        Box(
                            modifier = Modifier
                                .size(44.dp)
                                .border(
                                    width = 1.5.dp,
                                    color = if (char != null)
                                        MaterialTheme.colorScheme.primary
                                    else
                                        MaterialTheme.colorScheme.outline,
                                    shape = MaterialTheme.shapes.small
                                )
                                .testTag("password_cell_$index"),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = if (char != null) "●" else "",
                                style = MaterialTheme.typography.bodyLarge,
                                textAlign = TextAlign.Center
                            )
                        }
                    }
                }
            }
        )
    }
}
