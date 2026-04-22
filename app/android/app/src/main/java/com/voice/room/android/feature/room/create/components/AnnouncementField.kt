package com.voice.room.android.feature.room.create.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag

private const val MAX_ANNOUNCEMENT_LENGTH = 200

/**
 * 公告输入框（T-30036）
 *
 * 最多允许输入 200 字符，右下角实时显示字数计数。
 * 超出限制时计数器文字变红。
 *
 * @param value         当前公告文本
 * @param onValueChange 公告变化回调
 * @param modifier      可选 Modifier
 * @param enabled       是否可交互
 */
@Composable
fun AnnouncementField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true
) {
    val isOverLimit = value.length > MAX_ANNOUNCEMENT_LENGTH
    val counterColor = if (isOverLimit)
        MaterialTheme.colorScheme.error
    else
        MaterialTheme.colorScheme.onSurfaceVariant

    Column(modifier = modifier) {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            label = { Text("公告（可选）") },
            placeholder = { Text("填写房间公告，最多 $MAX_ANNOUNCEMENT_LENGTH 字符") },
            supportingText = {
                Text(
                    text = "${value.length}/$MAX_ANNOUNCEMENT_LENGTH",
                    color = counterColor,
                    style = MaterialTheme.typography.labelSmall,
                    modifier = Modifier.testTag("announcement_counter")
                )
            },
            isError = isOverLimit,
            maxLines = 4,
            enabled = enabled,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("announcement_input")
        )
    }
}
