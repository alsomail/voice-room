package com.voice.room.android.feature.room

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.util.Log
import android.widget.Toast
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors

/**
 * 聊天消息列表 (T-30014)
 *
 * 使用 [LazyColumn] 展示消息列表：
 *  - [MessageType.USER_TEXT]     → 左对齐，显示昵称 + 内容
 *  - [MessageType.SYSTEM_NOTICE] → 居中，灰色文字
 *
 * 新消息到来时自动滚动到底部（[LaunchedEffect] 监听列表长度变化）。
 * 内部对 [messages] 按 [ChatMessageUi.messageId] 去重，避免 LazyColumn key 冲突。
 *
 * testTag: `"chat_message_list"` (LazyColumn)
 * 子项 testTag: `"user_message_{index}"` / `"system_message_{index}"`
 *
 * @param messages 聊天消息列表
 * @param modifier 可选 Modifier
 */
@Composable
fun ChatMessageList(
    messages: List<ChatMessageUi>,
    modifier: Modifier = Modifier,
) {
    val listState = rememberLazyListState()

    // 去重：相同 messageId 只保留首条，防止 LazyColumn key 冲突
    val deduplicated = remember(messages) { messages.distinctBy { it.messageId } }

    // 自动滚到最新消息（底部）
    LaunchedEffect(deduplicated.size) {
        // T-30051: WS 接收链路可观测性 — 节点 5（UI 收集器）。
        Log.d("ChatMessageList", "ui: chatMessages collected size=${deduplicated.size}")
        if (deduplicated.isNotEmpty()) {
            listState.animateScrollToItem(deduplicated.lastIndex)
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier.testTag("chat_message_list"),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        itemsIndexed(items = deduplicated, key = { _, msg -> msg.messageId }) { index, message ->
            when (message.messageType) {
                MessageType.USER_TEXT -> UserMessageItem(
                    message = message,
                    modifier = Modifier
                        .semantics(mergeDescendants = true) { testTag = "user_message_$index" }
                        .fillMaxWidth(),
                )
                MessageType.SYSTEM_NOTICE -> SystemNoticeItem(
                    message = message,
                    modifier = Modifier
                        .semantics(mergeDescendants = true) { testTag = "system_message_$index" }
                        .fillMaxWidth(),
                )
            }
        }
    }
}

/**
 * 用户消息条目：左对齐，昵称（金色 MenaColors.Primary）+ 内容气泡
 *
 * T-30025: 昵称色改为 MenaColors.Primary (#D4AF37 金色)
 * T-30052: 内容外包 Surface 气泡（圆角 + MenaColors.ChatBubble 背景 + padding），
 *          并加 testTag("chat_bubble") 供 Midscene 视觉 AI / 测试识别。
 * T-30053: 长按弹出 DropdownMenu 含「复制」选项，点击后写入 ClipboardManager + Toast
 *          可见性 internal 以便同模块 androidTest 直接调用。
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun UserMessageItem(
    message: ChatMessageUi,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    var showMenu by remember { mutableStateOf(false) }

    Box(
        modifier = modifier.combinedClickable(
            onClick = {},
            onLongClick = { showMenu = true },
        ),
    ) {
        Column {
            if (message.senderNickname != null) {
                Text(
                    text = message.senderNickname,
                    style = MaterialTheme.typography.labelSmall,
                    color = MenaColors.Primary,   // T-30025: #D4AF37 金色
                )
            }
            Surface(
                modifier = Modifier
                    .widthIn(max = 280.dp)
                    .testTag("chat_bubble"),
                shape = MaterialTheme.shapes.medium,
                color = MenaColors.ChatBubble,    // T-30052: #2A2A2A 气泡背景
            ) {
                Text(
                    text = message.content,
                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }

        DropdownMenu(
            expanded = showMenu,
            onDismissRequest = { showMenu = false },
            modifier = Modifier.semantics { contentDescription = "chat_msg_long_press_menu" },
        ) {
            DropdownMenuItem(
                text = { Text(stringResource(R.string.chat_msg_copy)) },
                onClick = {
                    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    val clip = ClipData.newPlainText("message", message.content)
                    clipboard.setPrimaryClip(clip)
                    // Android 13+(API 33 TIRAMISU) 系统会自动显示剪贴板提示，避免双重 Toast
                    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
                        Toast.makeText(context, context.getString(R.string.chat_msg_copy_success), Toast.LENGTH_SHORT).show()
                    }
                    showMenu = false
                },
                modifier = Modifier
                    .testTag("chat_msg_copy")
                    .semantics { contentDescription = "chat_msg_copy" },
            )
        }
    }
}

/**
 * 系统通知条目：居中，金黄色（MenaColors.SystemMessage），无昵称头像
 *
 * T-30025: 颜色改为 MenaColors.SystemMessage (#F39C12 金黄色)
 */
@Composable
private fun SystemNoticeItem(
    message: ChatMessageUi,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier,
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = message.content,
            style = MaterialTheme.typography.labelSmall,
            color = MenaColors.SystemMessage,   // T-30025: #F39C12 金黄色
            textAlign = TextAlign.Center,
        )
    }
}

// ─────────────────────────────────────────────
// Previews
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "ChatMessageList — 混合消息预览")
@Composable
private fun ChatMessageListPreview() {
    ChatMessageList(
        messages = listOf(
            ChatMessageUi(messageId = "m1", senderNickname = "Alice", content = "大家好！", timestamp = 0L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s1", senderNickname = null, content = "Bob 进入了房间", timestamp = 1L, messageType = MessageType.SYSTEM_NOTICE),
            ChatMessageUi(messageId = "m2", senderNickname = "Bob", content = "欢迎~", timestamp = 2L, messageType = MessageType.USER_TEXT),
            ChatMessageUi(messageId = "s2", senderNickname = null, content = "Carol 进入了房间", timestamp = 3L, messageType = MessageType.SYSTEM_NOTICE),
            ChatMessageUi(messageId = "m3", senderNickname = "Carol", content = "今天天气不错", timestamp = 4L, messageType = MessageType.USER_TEXT),
        ),
    )
}

