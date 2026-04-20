package com.voice.room.android.feature.auth.components

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

/**
 * 手机号输入组件
 *
 * - 左侧（RTL 时为右侧）显示固定国家码前缀（默认 +966）
 * - 输入框仅接受数字键盘
 * - 最多输入 9 位（沙特本机号）
 */
@Composable
fun PhoneInput(
    phoneNumber: String,
    onPhoneNumberChanged: (String) -> Unit,
    modifier: Modifier = Modifier,
    countryCode: String = "+966",
    enabled: Boolean = true
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // 国家码前缀标签
        Surface(
            modifier = Modifier
                .border(
                    width = 1.dp,
                    color = MaterialTheme.colorScheme.outline,
                    shape = MaterialTheme.shapes.extraSmall
                )
                .padding(horizontal = 12.dp, vertical = 18.dp)
        ) {
            Text(
                text = countryCode,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface
            )
        }

        Spacer(modifier = Modifier.width(8.dp))

        // 手机号输入框
        OutlinedTextField(
            value = phoneNumber,
            onValueChange = { newValue ->
                // 仅保留数字，最多 9 位
                val digits = newValue.filter { it.isDigit() }.take(9)
                onPhoneNumberChanged(digits)
            },
            modifier = Modifier.weight(1f),
            enabled = enabled,
            singleLine = true,
            placeholder = {
                Text(text = "5XXXXXXXX")
            },
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Number
            ),
            label = {
                Text(text = "رقم الهاتف")  // 阿拉伯语"手机号"
            }
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun PhoneInputPreview() {
    MaterialTheme {
        PhoneInput(
            phoneNumber = "501234567",
            onPhoneNumberChanged = {}
        )
    }
}
