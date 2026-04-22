package com.voice.room.android.feature.gift.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.AvatarWithFrame
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography
import com.voice.room.android.domain.gift.MicUserVO

/**
 * 接收者选择器 (T-30029)
 *
 * 在礼物面板顶部展示横向滚动的麦位用户头像条，用于选择送礼接收者。
 *
 * - 非空列表：[LazyRow] 横向滚动，每项 [AvatarWithFrame](48dp) + 昵称省略号
 * - 主麦（slot=0）置首（由 [GiftPanelViewModel.updateRecipients] 排序保证）
 * - 选中项：金色 2dp 光圈边框 + 底部实心金色圆点
 * - 空列表：显示"当前无人在麦"（居中灰字），`selectedRecipientId` 为 null
 * - 首次渲染若 selected 为 null 且有列表，ViewModel 已自动选中第一个（主麦）
 *
 * ### testTag 协议（TDS §testTag）
 * - 容器：          `recipient_selector`
 * - 每项：          `recipient_item_{userId}`
 * - 空状态文本：    `recipient_empty`
 *
 * @param recipients 仅 on-mic 用户列表（已按 micIndex 升序排序，slot=0 在首位）
 * @param selectedId 当前选中的用户 ID（null = 未选中）
 * @param onSelect   点击头像时的回调，传出被点击用户的 userId
 * @param modifier   可选 Modifier
 */
@Composable
fun RecipientSelector(
    recipients: List<MicUserVO>,
    selectedId: String?,
    onSelect: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .testTag("recipient_selector"),
    ) {
        if (recipients.isEmpty()) {
            // ── 空状态：无人在麦 ──────────────────────────────────────────────
            Text(
                text = "当前无人在麦",
                style = MenaTypography.bodyMedium,
                color = MenaColors.OnBackgroundTertiary,
                modifier = Modifier
                    .align(Alignment.Center)
                    .testTag("recipient_empty")
                    .padding(vertical = 12.dp),
            )
        } else {
            // ── 横向滚动头像条 ────────────────────────────────────────────────
            LazyRow(
                contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                items(recipients, key = { it.userId }) { user ->
                    RecipientItem(
                        user = user,
                        isSelected = user.userId == selectedId,
                        onSelect = onSelect,
                    )
                }
            }
        }
    }
}

/**
 * 单个接收者头像项
 *
 * - 48dp 头像（[AvatarWithFrame]）
 * - 选中态：金色 2dp 边框 + 底部实心 4dp 金色圆点
 * - 昵称最多 1 行，超出省略号
 *
 * testTag：`recipient_item_{userId}`
 */
@Composable
private fun RecipientItem(
    user: MicUserVO,
    isSelected: Boolean,
    onSelect: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .width(56.dp)
            .clickable { onSelect(user.userId) }
            .testTag("recipient_item_${user.userId}"),
        contentAlignment = Alignment.TopCenter,
    ) {
        Column(
            modifier = Modifier.align(Alignment.BottomCenter),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            // ── 头像 + 选中边框 ────────────────────────────────────────────────
            AvatarWithFrame(
                imageUrl = user.avatarUrl,
                size = 48.dp,
                showFrame = isSelected,
            )

            // ── 昵称（省略号截断） ────────────────────────────────────────────
            Text(
                text = user.nickname,
                style = MenaTypography.labelSmall,
                color = if (isSelected) MenaColors.Primary else MenaColors.OnBackgroundSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        // 底部实心金色选中指示点（在头像 Column 外部，避免昵称文字跳动）
        if (isSelected) {
            Box(
                modifier = Modifier
                    .size(4.dp)
                    .background(MenaColors.Primary, CircleShape)
                    .align(Alignment.TopCenter),
            )
        }
    }
}

// ─── Preview ─────────────────────────────────────────────────────────────────

@Preview(showBackground = true, backgroundColor = 0xFF1A1A2E, name = "RecipientSelector — 有人在麦")
@Composable
private fun RecipientSelectorPreview() {
    RecipientSelector(
        recipients = listOf(
            MicUserVO(userId = "host", nickname = "Alice（主麦）", avatarUrl = null, micIndex = 0),
            MicUserVO(userId = "u2", nickname = "Bob", avatarUrl = null, micIndex = 2),
            MicUserVO(userId = "u5", nickname = "Charlie的昵称很长", avatarUrl = null, micIndex = 5),
        ),
        selectedId = "host",
        onSelect = {},
    )
}

@Preview(showBackground = true, backgroundColor = 0xFF1A1A2E, name = "RecipientSelector — 无人在麦")
@Composable
private fun RecipientSelectorEmptyPreview() {
    RecipientSelector(
        recipients = emptyList(),
        selectedId = null,
        onSelect = {},
    )
}
