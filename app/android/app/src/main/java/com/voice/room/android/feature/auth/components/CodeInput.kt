package com.voice.room.android.feature.auth.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview
import com.voice.room.android.core.theme.GoldOutlinedTextField
import com.voice.room.android.core.theme.MenaTheme

/**
 * 验证码输入组件
 *
 * - 纯数字键盘
 * - 最多 6 位
 * - 满 6 位后自动禁止继续输入
 */
@Composable
@Suppress("UNUSED_PARAMETER")
fun CodeInput(
    code: String,
    onCodeChanged: (String) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true  // 保留 API 兼容性，GoldOutlinedTextField 暂不支持 enabled
) {
    GoldOutlinedTextField(
        value = code,
        onValueChange = { newValue ->
            // 仅保留数字，最多 6 位
            val digits = newValue.filter { it.isDigit() }.take(6)
            onCodeChanged(digits)
        },
        modifier = modifier.fillMaxWidth(),
        label = "رمز التحقق",
        placeholder = "------",
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.NumberPassword
        ),
    )
}

@Preview(showBackground = true)
@Composable
private fun CodeInputPreview() {
    MenaTheme {
        CodeInput(code = "", onCodeChanged = {})
    }
}
