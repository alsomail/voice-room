package com.voice.room.android.feature.room

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.CardGiftcard
import androidx.compose.material.icons.filled.EmojiEmotions
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import com.voice.room.android.core.theme.GoldOutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.R
import com.voice.room.android.core.theme.MenaColors

/**
 * 房间底部操作栏（T-30026）
 *
 * 布局：Row — [GoldOutlinedTextField(weight=1f)] + [MicButton] + [GiftButton] + [EmoteButton] + [ExitButton]
 * 背景：MenaColors.Surface，上边框：1dp MenaColors.SurfaceVariant
 *
 * testTag 协议：
 *   - 容器：    "room_bottom_bar"
 *   - 输入框：  "chat_input_field"（沿用原 ChatInputBar 协议）
 *   - 发送按钮："chat_send_button"（沿用）
 *   - 麦克风：  "btn_mic_toggle"
 *   - 礼物：    "btn_gift"
 *   - 表情：    "btn_emoji"
 *   - 退出：    "btn_exit_room"
 *   - 退出弹窗："exit_room_dialog"
 *
 * @param inputText           当前输入框文字
 * @param onInputTextChange   输入变化回调
 * @param isSending           发送中禁用标志
 * @param onSendMessage       发送回调
 * @param isOnMic             当前用户是否在麦上
 * @param isMicMuted          当前用户麦克风是否静音
 * @param onMicToggle         点击麦克风按钮的回调（toggle 静音）
 * @param onLeaveRoom         确认退出房间的回调
 * @param onEmojiClick        点击 😊 表情按钮的回调（缺陷 #2 修复：替换原 Composable 内 Toast）
 * @param onGiftClick         点击 🎁 礼物按钮的回调（T-30028）
 * @param modifier            可选 Modifier
 */
@Composable
fun RoomBottomBar(
    inputText: String,
    onInputTextChange: (String) -> Unit,
    isSending: Boolean,
    onSendMessage: (String) -> Unit,
    isOnMic: Boolean,
    isMicMuted: Boolean,
    onMicToggle: () -> Unit,
    onLeaveRoom: () -> Unit,
    onGiftClick: () -> Unit = {},   // T-30028: 🎁 按钮点击回调（替换 Toast 占位）
    onEmojiClick: () -> Unit = {},  // 缺陷 #2 修复：表情点击交由调用方处理（不在 Composable 内 Toast）
    modifier: Modifier = Modifier,
) {
    val canSend = inputText.isNotBlank() && !isSending

    // 退出确认弹窗状态
    var showExitDialog by remember { mutableStateOf(false) }

    // 麦克风颜色：不在麦上→灰，在麦+开麦→绿，在麦+静音→红
    val micTint = when {
        !isOnMic   -> MenaColors.OnBackgroundTertiary
        isMicMuted -> MenaColors.Error
        else       -> MenaColors.Success
    }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .testTag("room_bottom_bar")
            .border(width = 1.dp, color = MenaColors.SurfaceVariant)
            .padding(horizontal = 8.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // ── 输入框（沿用 ChatInputBar 的 testTag 协议） ──────────────────────
        GoldOutlinedTextField(
            value = inputText,
            onValueChange = onInputTextChange,
            modifier = Modifier
                .weight(1f)
                .testTag("chat_input_field"),
            placeholder = stringResource(id = R.string.room_input_placeholder),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
            keyboardActions = KeyboardActions(
                onSend = { if (canSend) onSendMessage(inputText) }
            ),
            singleLine = true,
        )

        // ── 发送按钮 ────────────────────────────────────────────────────────
        IconButton(
            onClick = { onSendMessage(inputText) },
            enabled = canSend,
            modifier = Modifier.testTag("chat_send_button"),
        ) {
            Icon(
                imageVector = Icons.AutoMirrored.Filled.Send,
                contentDescription = stringResource(id = R.string.room_send_action),
                tint = if (canSend) MenaColors.Primary else MenaColors.OnBackgroundTertiary,
            )
        }

        Spacer(modifier = Modifier.width(2.dp))

        // ── 麦克风按钮 ───────────────────────────────────────────────────────
        // enabled=isOnMic：不在麦上时视觉灰色 + 禁用点击
        IconButton(
            onClick = onMicToggle,
            enabled = isOnMic,
            modifier = Modifier.testTag("btn_mic_toggle"),
        ) {
            Icon(
                imageVector = if (isMicMuted) Icons.Default.MicOff else Icons.Default.Mic,
                contentDescription = stringResource(
                    id = if (isMicMuted) R.string.room_unmute_action else R.string.room_mute_action
                ),
                tint = micTint,
            )
        }

        // ── 礼物按钮（T-30028: 点击弹出 GiftPanelBottomSheet） ───────────────
        IconButton(
            onClick = onGiftClick,
            enabled = true,
            modifier = Modifier.testTag("btn_gift"),
        ) {
            Icon(
                imageVector = Icons.Default.CardGiftcard,
                contentDescription = stringResource(id = R.string.room_gift_action),
                tint = MenaColors.Primary,  // T-30028: 礼物按钮激活为金色
            )
        }

        // ── 表情按钮（缺陷 #2 修复：交由调用方处理，不在 Composable 内 Toast） ──
        IconButton(
            onClick = onEmojiClick,
            enabled = true,
            modifier = Modifier.testTag("btn_emoji"),
        ) {
            Icon(
                imageVector = Icons.Default.EmojiEmotions,
                contentDescription = stringResource(id = R.string.room_emoji_action),
                tint = MenaColors.OnBackgroundTertiary,
            )
        }

        // ── 退出按钮 ─────────────────────────────────────────────────────────
        IconButton(
            onClick = { showExitDialog = true },
            modifier = Modifier.testTag("btn_exit_room"),
        ) {
            Icon(
                imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                contentDescription = stringResource(id = R.string.room_exit_action),
                tint = MenaColors.Error,
            )
        }
    }

    // ── 退出确认弹窗 ─────────────────────────────────────────────────────────
    if (showExitDialog) {
        AlertDialog(
            modifier = Modifier.testTag("exit_room_dialog"),
            onDismissRequest = { showExitDialog = false },
            title = { Text(stringResource(id = R.string.room_exit_dialog_title)) },
            text = { Text(stringResource(id = R.string.room_exit_dialog_text)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        showExitDialog = false
                        onLeaveRoom()
                    }
                ) {
                    Text(stringResource(id = R.string.dialog_confirm), color = MenaColors.Error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showExitDialog = false }) {
                    Text(stringResource(id = R.string.dialog_cancel))
                }
            },
        )
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "RoomBottomBar — 不在麦上")
@Composable
private fun RoomBottomBarOffMicPreview() {
    RoomBottomBar(
        inputText = "",
        onInputTextChange = {},
        isSending = false,
        onSendMessage = {},
        isOnMic = false,
        isMicMuted = false,
        onMicToggle = {},
        onLeaveRoom = {},
    )
}

@Preview(showBackground = true, name = "RoomBottomBar — 在麦开麦（绿色）")
@Composable
private fun RoomBottomBarOnMicOpenPreview() {
    RoomBottomBar(
        inputText = "大家好",
        onInputTextChange = {},
        isSending = false,
        onSendMessage = {},
        isOnMic = true,
        isMicMuted = false,
        onMicToggle = {},
        onLeaveRoom = {},
    )
}

@Preview(showBackground = true, name = "RoomBottomBar — 在麦静音（红色）")
@Composable
private fun RoomBottomBarOnMicMutedPreview() {
    RoomBottomBar(
        inputText = "",
        onInputTextChange = {},
        isSending = false,
        onSendMessage = {},
        isOnMic = true,
        isMicMuted = true,
        onMicToggle = {},
        onLeaveRoom = {},
    )
}
