package com.voice.room.android.feature.room

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.voice.room.android.R
import com.voice.room.android.core.theme.MenaColors

/**
 * 麦位区域布局 (T-30009 / T-30025)
 *
 * 布局结构（黑金风格升级）：
 * - 主麦行：[HostMicSlot] 水平居中（slots[0]，80dp 金色光圈）
 * - 副麦网格：[LazyVerticalGrid] 固定 4 列（slots[1..8]，60dp AvatarWithFrame）
 *
 * 背景色：[MenaColors.Background]（深色黑金底色）
 * 高度：`wrapContentHeight()`（移除旧版 height(240.dp) 硬编码）
 *
 * testTag: `"mic_slots_grid"` — 副麦 LazyVerticalGrid 容器
 *
 * @param slots           最多 9 个麦位 UI 状态列表（兼容旧调用方）
 * @param modifier        可选 Modifier
 * @param onMicSlotClick  麦位点击回调，参数为 [MicSlotUi.index]
 */
@Composable
fun MicSlotsGrid(
    slots: List<MicSlotUi>,
    modifier: Modifier = Modifier,
    onMicSlotClick: (index: Int) -> Unit = {},
) {
    val hostSlot = slots.getOrNull(0)
    val guestSlots = if (slots.size > 1) slots.drop(1) else emptyList()

    Column(
        modifier = modifier
            .fillMaxWidth()
            .background(MenaColors.Background),
    ) {
        // ── 主麦行：居中显示 HostMicSlot ────────────────────────────────────
        if (hostSlot != null) {
            Box(
                modifier = Modifier.fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                HostMicSlot(slot = hostSlot, onClick = onMicSlotClick)
            }
        }

        // ── 副麦区：4 列 LazyVerticalGrid，userScrollEnabled=false ──────────
        LazyVerticalGrid(
            columns = GridCells.Fixed(4),
            modifier = Modifier
                .fillMaxWidth()
                .testTag("mic_slots_grid"),
            userScrollEnabled = false,
        ) {
            items(items = guestSlots, key = { it.index }) { slot ->
                MicSlotCard(slot = slot, onClick = onMicSlotClick)
            }
        }
    }
}

/**
 * 单个麦位 Composable (T-30009)
 *
 * 保留原有实现（兼容性），被 MicSlotsGrid 使用。
 * 新代码优先使用 MicSlotCard（T-30011）。
 *
 * 两种状态：
 * - **空麦**（`userId == null`）：麦克风图标 + "空位"文本
 *   `testTag("mic_slot_empty_${slot.index}")`
 * - **有人**（`userId != null`）：Coil AsyncImage 头像 + 昵称 + 静音图标（若 `isMuted`）
 *   `testTag("mic_slot_occupied_${slot.index}")`
 *   静音图标：`testTag("mic_slot_muted_icon")`
 *
 * @param slot     麦位 UI 状态
 * @param modifier 可选 Modifier
 */
@Composable
fun MicSlotItem(
    slot: MicSlotUi,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .padding(8.dp)
            .testTag(
                if (slot.isOccupied) "mic_slot_occupied_${slot.index}"
                else "mic_slot_empty_${slot.index}"
            ),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        if (slot.isOccupied) {
            // 有人：头像 + 昵称 + 静音图标
            AsyncImage(
                model = slot.avatarUrl,
                contentDescription = slot.nickname,
                placeholder = androidx.compose.ui.res.painterResource(R.drawable.ic_placeholder),
                fallback = androidx.compose.ui.res.painterResource(R.drawable.ic_placeholder),
                modifier = Modifier.size(48.dp),
            )
            Text(
                text = slot.nickname ?: "",
                style = MaterialTheme.typography.bodySmall,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
            if (slot.isMuted) {
                Icon(
                    imageVector = Icons.Default.MicOff,
                    contentDescription = "静音",
                    modifier = Modifier
                        .size(16.dp)
                        .testTag("mic_slot_muted_icon"),
                    tint = MaterialTheme.colorScheme.error,
                )
            }
        } else {
            // 空麦：麦克风占位图标 + "空位"文本
            Icon(
                imageVector = Icons.Default.MicOff,
                contentDescription = "空位",
                modifier = Modifier.size(48.dp),
                tint = MaterialTheme.colorScheme.outline,
            )
            Text(
                text = "空位",
                style = MaterialTheme.typography.bodySmall,
                textAlign = TextAlign.Center,
            )
        }
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, backgroundColor = 0xFF1A1A2E, name = "MicSlotsGrid — 黑金风格")
@Composable
private fun MicSlotsGridPreview() {
    val slots = List(9) { index ->
        when (index) {
            0 -> MicSlotUi(index = 0, userId = "u1", nickname = "Alice")
            1 -> MicSlotUi(index = 1, userId = "u2", nickname = "Bob", isMuted = true)
            else -> MicSlotUi(index = index)
        }
    }
    MicSlotsGrid(slots = slots)
}
