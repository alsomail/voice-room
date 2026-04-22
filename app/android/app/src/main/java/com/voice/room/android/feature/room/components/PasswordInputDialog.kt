package com.voice.room.android.feature.room.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.voice.room.android.feature.room.PasswordDialogState

private const val PASSWORD_LENGTH = 6

/**
 * 密码房进房弹窗（T-30038）
 *
 * - 6 个单字符分格输入，输完自动 submit
 * - state=Verifying：全部输入框不可编辑 + 显示 loading 指示器
 * - state=Error：底部红字"密码错误，剩余 N 次"
 * - state=Locked：弹窗变只读 + "已被锁定，{remaining_min} 分钟后重试"
 * - 返回键 / 取消按钮 → onDismiss
 *
 * @param state      当前弹窗状态（由 HallViewModel.passwordDialogState 驱动）
 * @param onSubmit   6 位密码输完时的回调（密码字符串）
 * @param onDismiss  关闭弹窗回调
 */
@Composable
fun PasswordInputDialog(
    state: PasswordDialogState,
    onSubmit: (String) -> Unit,
    onDismiss: () -> Unit
) {
    val digits = remember { mutableStateListOf(*Array(PASSWORD_LENGTH) { "" }) }
    val focusRequesters = remember { List(PASSWORD_LENGTH) { FocusRequester() } }

    val isReadOnly = state is PasswordDialogState.Verifying || state is PasswordDialogState.Locked

    // 每次弹窗出现（state 切换为 Idle）时清空输入并聚焦第 0 格
    LaunchedEffect(state is PasswordDialogState.Idle) {
        if (state is PasswordDialogState.Idle) {
            digits.fill("")
            focusRequesters[0].requestFocus()
        }
    }

    // 密码错误后清空输入，重新聚焦
    LaunchedEffect(state is PasswordDialogState.Error) {
        if (state is PasswordDialogState.Error) {
            digits.fill("")
            focusRequesters[0].requestFocus()
        }
    }

    AlertDialog(
        modifier = Modifier.testTag("password_dialog"),
        onDismissRequest = { if (!isReadOnly) onDismiss() },
        title = { Text("请输入房间密码") },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                // 6 格密码输入行
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("password_input"),
                    horizontalArrangement = Arrangement.SpaceEvenly
                ) {
                    digits.forEachIndexed { index, value ->
                        OutlinedTextField(
                            value = value,
                            onValueChange = { newVal ->
                                if (isReadOnly) return@OutlinedTextField
                                val char = newVal.lastOrNull()?.toString() ?: ""
                                if (char.length > 1) return@OutlinedTextField // 过滤粘贴多字符
                                digits[index] = char
                                when {
                                    char.isNotEmpty() && index < PASSWORD_LENGTH - 1 ->
                                        focusRequesters[index + 1].requestFocus()
                                    char.isNotEmpty() && index == PASSWORD_LENGTH - 1 -> {
                                        // 最后一格输完：自动提交
                                        val pwd = digits.joinToString("")
                                        if (pwd.length == PASSWORD_LENGTH) {
                                            onSubmit(pwd)
                                        }
                                    }
                                    char.isEmpty() && index > 0 ->
                                        focusRequesters[index - 1].requestFocus()
                                }
                            },
                            modifier = Modifier
                                .width(44.dp)
                                .testTag("password_digit_$index")
                                .focusRequester(focusRequesters[index]),
                            singleLine = true,
                            enabled = !isReadOnly,
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                // 状态反馈文字
                when (state) {
                    is PasswordDialogState.Verifying ->
                        CircularProgressIndicator(modifier = Modifier.padding(4.dp))

                    is PasswordDialogState.Error ->
                        Text(
                            text = "密码错误，剩余 ${state.remainingAttempts} 次",
                            color = Color.Red,
                            modifier = Modifier.testTag("password_error_text")
                        )

                    is PasswordDialogState.Locked ->
                        Text(
                            text = "已被锁定，${state.remainingMinutes} 分钟后重试",
                            color = Color.Red,
                            modifier = Modifier.testTag("password_error_text")
                        )

                    else -> {}
                }
            }
        },
        confirmButton = {
            Button(
                modifier = Modifier.testTag("btn_submit_password"),
                enabled = !isReadOnly && digits.joinToString("").length == PASSWORD_LENGTH,
                onClick = { onSubmit(digits.joinToString("")) }
            ) {
                Text("确认")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isReadOnly) {
                Text("取消")
            }
        }
    )
}
