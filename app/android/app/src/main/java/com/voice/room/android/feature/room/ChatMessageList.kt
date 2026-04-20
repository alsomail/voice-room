package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

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
 * 用户消息条目：左对齐，昵称（primary 色）+ 内容（bodyMedium）
 */
@Composable
private fun UserMessageItem(
    message: ChatMessageUi,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier) {
        Column {
            if (message.senderNickname != null) {
                Text(
                    text = message.senderNickname,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            Text(
                text = message.content,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
    }
}

/**
 * 系统通知条目：居中，灰色文字（onSurface 60%），无昵称头像
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
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
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

