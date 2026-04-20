package com.voice.room.android.feature.auth.components

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.GoldOutlinedTextField
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTheme

/**
 * 手机号输入组件
 *
 * - 左侧（RTL 时为右侧）显示固定国家码前缀（默认 +966）
 * - 输入框仅接受数字键盘
 * - 最多输入 9 位（沙特本机号）
 */
@Composable
@Suppress("UNUSED_PARAMETER")
fun PhoneInput(
    phoneNumber: String,
    onPhoneNumberChanged: (String) -> Unit,
    modifier: Modifier = Modifier,
    countryCode: String = "+966",
    enabled: Boolean = true  // 保留 API 兼容性，GoldOutlinedTextField 暂不支持 enabled
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // 国家码前缀标签
        Surface(
            color = MenaColors.Surface,
            modifier = Modifier
                .border(
                    width = 1.dp,
                    color = MenaColors.Primary.copy(alpha = 0.5f),
                    shape = MaterialTheme.shapes.extraSmall
                )
                .padding(horizontal = 12.dp, vertical = 18.dp)
        ) {
            Text(
                text = countryCode,
                style = MaterialTheme.typography.bodyLarge,
                color = MenaColors.OnBackground
            )
        }

        Spacer(modifier = Modifier.width(8.dp))

        // 手机号输入框
        GoldOutlinedTextField(
            value = phoneNumber,
            onValueChange = { newValue ->
                // 仅保留数字，最多 9 位
                val digits = newValue.filter { it.isDigit() }.take(9)
                onPhoneNumberChanged(digits)
            },
            modifier = Modifier.weight(1f),
            label = "رقم الهاتف",
            placeholder = "5XXXXXXXX",
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Number
            ),
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun PhoneInputPreview() {
    MenaTheme {
        PhoneInput(
            phoneNumber = "501234567",
            onPhoneNumberChanged = {}
        )
    }
}
