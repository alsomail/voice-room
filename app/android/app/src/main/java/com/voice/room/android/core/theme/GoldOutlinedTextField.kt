package com.voice.room.android.core.theme

import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * GoldOutlinedTextField — 深色底 + 金色边框输入框
 *
 * - 深色背景 (Surface)
 * - 金色边框 1dp（unfocused: Primary 50% 透明度，focused: PrimaryBright 100%）
 * - 白色输入文字 (OnBackground)
 * - 金色 label/placeholder (Primary)
 * - 12dp 圆角
 *
 * @param value           当前文本
 * @param onValueChange   文本变化回调
 * @param label           标签文本
 * @param modifier        外部 Modifier
 * @param placeholder     占位文本
 * @param singleLine      单行模式
 * @param keyboardOptions 键盘选项
 * @param keyboardActions 键盘动作
 */
@Composable
fun GoldOutlinedTextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    label: String = "",
    placeholder: String = "",
    singleLine: Boolean = true,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier,
        label = if (label.isNotEmpty()) {
            { Text(text = label) }
        } else {
            null
        },
        placeholder = if (placeholder.isNotEmpty()) {
            { Text(text = placeholder) }
        } else {
            null
        },
        singleLine = singleLine,
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
        shape = MenaShapes.small,
        colors = OutlinedTextFieldDefaults.colors(
            // 文本颜色
            focusedTextColor = MenaColors.OnBackground,
            unfocusedTextColor = MenaColors.OnBackground,
            // 容器背景
            focusedContainerColor = MenaColors.Surface,
            unfocusedContainerColor = MenaColors.Surface,
            // 边框颜色
            focusedBorderColor = MenaColors.PrimaryBright,
            unfocusedBorderColor = MenaColors.Primary.copy(alpha = 0.5f),
            // Label 颜色
            focusedLabelColor = MenaColors.PrimaryBright,
            unfocusedLabelColor = MenaColors.Primary,
            // Placeholder 颜色
            focusedPlaceholderColor = MenaColors.Primary,
            unfocusedPlaceholderColor = MenaColors.Primary,
            // 光标颜色
            cursorColor = MenaColors.PrimaryBright,
        ),
    )
}
