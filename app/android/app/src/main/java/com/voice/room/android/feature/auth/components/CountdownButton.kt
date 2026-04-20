package com.voice.room.android.feature.auth.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

/**
 * 发送验证码 / 倒计时按钮
 *
 * 状态：
 * - 可用：显示"إرسال رمز التحقق"（发送验证码）
 * - 倒计时中：显示剩余秒数，按钮禁用
 */
@Composable
fun CountdownButton(
    isEnabled: Boolean,
    isCountingDown: Boolean,
    countdownLabel: String,
    onSendCode: () -> Unit,
    modifier: Modifier = Modifier
) {
    Button(
        onClick = onSendCode,
        enabled = isEnabled,
        modifier = modifier
            .fillMaxWidth()
            .height(52.dp)
    ) {
        Text(
            text = if (isCountingDown) countdownLabel else "إرسال رمز التحقق",
            style = MaterialTheme.typography.labelLarge
        )
    }
}

@Preview(showBackground = true, name = "CountdownButton - Enabled")
@Composable
private fun CountdownButtonEnabledPreview() {
    MaterialTheme {
        CountdownButton(
            isEnabled = true,
            isCountingDown = false,
            countdownLabel = "",
            onSendCode = {}
        )
    }
}

@Preview(showBackground = true, name = "CountdownButton - Counting")
@Composable
private fun CountdownButtonCountingPreview() {
    MaterialTheme {
        CountdownButton(
            isEnabled = false,
            isCountingDown = true,
            countdownLabel = "42s",
            onSendCode = {}
        )
    }
}
