package com.voice.room.android.feature.auth.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview

/**
 * 验证码输入组件
 *
 * - 纯数字键盘
 * - 最多 6 位
 * - 满 6 位后自动禁止继续输入
 */
@Composable
fun CodeInput(
    code: String,
    onCodeChanged: (String) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true
) {
    OutlinedTextField(
        value = code,
        onValueChange = { newValue ->
            // 仅保留数字，最多 6 位
            val digits = newValue.filter { it.isDigit() }.take(6)
            onCodeChanged(digits)
        },
        modifier = modifier.fillMaxWidth(),
        enabled = enabled,
        singleLine = true,
        placeholder = {
            Text(text = "------")
        },
        label = {
            Text(text = "رمز التحقق")  // 阿拉伯语"验证码"
        },
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.NumberPassword
        )
    )
}

@Preview(showBackground = true)
@Composable
private fun CodeInputPreview() {
    MaterialTheme {
        CodeInput(code = "", onCodeChanged = {})
    }
}
