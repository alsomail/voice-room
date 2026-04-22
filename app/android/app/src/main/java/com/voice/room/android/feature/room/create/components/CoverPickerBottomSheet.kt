package com.voice.room.android.feature.room.create.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetState
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.feature.room.create.COVER_GOLD_BORDER_COLOR
import com.voice.room.android.feature.room.create.COVER_GOLD_BORDER_WIDTH_DP
import com.voice.room.android.feature.room.create.COVER_OPTIONS
import com.voice.room.android.feature.room.create.CoverPickerState

// ─────────────────────────────────────────────────────────────────────────────
// 封面选择器 BottomSheet（T-30037）
// ─────────────────────────────────────────────────────────────────────────────

private val GoldBorderColor = Color(COVER_GOLD_BORDER_COLOR)
private val GoldBorderWidth = COVER_GOLD_BORDER_WIDTH_DP.dp
private val ItemShape = RoundedCornerShape(8.dp)

/**
 * CoverPickerBottomSheet — 封面选择器模态底部面板
 *
 * 布局：
 * - [ModalBottomSheet]（expanded）
 * - [LazyVerticalGrid] 3 列，每格正方形（aspectRatio=1）
 * - 选中项 2dp 金色边框（0xFFD4AF37）
 * - 底部 [GoldButton] 点击后调用 [onCoverSelected]
 *
 * testTag 约定（用于 Instrumented UI 测试）：
 * - `cover_picker_sheet`  — 整个 BottomSheet 容器
 * - `cover_option_0`~`cover_option_7` — 各封面格
 * - `btn_confirm_cover`  — 确认按钮
 *
 * @param onCoverSelected  用户确认选中后传出封面 URL 的回调
 * @param onDismiss        关闭面板的回调
 * @param sheetState       可选，外部控制面板展开/收起状态
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CoverPickerBottomSheet(
    onCoverSelected: (String) -> Unit,
    onDismiss: () -> Unit,
    sheetState: SheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
) {
    // 纯状态持有者，与 Composable 生命周期绑定
    val state = remember {
        CoverPickerState(onCoverSelected = onCoverSelected)
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = Modifier.testTag("cover_picker_sheet"),
    ) {
        // ── 3 列封面网格 ──────────────────────────────────────────────────────
        LazyVerticalGrid(
            columns = GridCells.Fixed(3),
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp),
        ) {
            itemsIndexed(COVER_OPTIONS) { index, option ->
                CoverGridItem(
                    resId = option.resId,
                    isSelected = state.selectedIndex == index,
                    tag = "cover_option_$index",
                    onClick = { state.selectCover(index) },
                    modifier = Modifier.padding(4.dp),
                )
            }
        }

        // ── 确认按钮 ──────────────────────────────────────────────────────────
        GoldButton(
            text = "确认",
            onClick = { state.confirmSelection() },
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 16.dp)
                .testTag("btn_confirm_cover"),
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 单个封面格
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 单个封面选项格
 *
 * @param resId      Drawable 资源 ID
 * @param isSelected 当前是否选中（决定是否显示金色边框）
 * @param tag        testTag 字符串
 * @param onClick    点击回调
 * @param modifier   外部 Modifier
 */
@Composable
private fun CoverGridItem(
    resId: Int,
    isSelected: Boolean,
    tag: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val borderModifier = if (isSelected) {
        Modifier.border(
            border = BorderStroke(GoldBorderWidth, GoldBorderColor),
            shape = ItemShape,
        )
    } else {
        Modifier
    }

    Box(
        modifier = modifier
            .aspectRatio(1f)
            .clip(ItemShape)
            .then(borderModifier)
            .clickable(onClick = onClick)
            .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Image(
            painter = painterResource(id = resId),
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize(),
        )
    }
}
