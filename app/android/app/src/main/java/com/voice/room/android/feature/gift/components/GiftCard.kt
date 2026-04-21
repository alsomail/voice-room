package com.voice.room.android.feature.gift.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography
import com.voice.room.android.domain.gift.GiftVO

/**
 * 4 列网格中的单个礼物卡片 (T-30028)
 *
 * - 未选中：`MenaColors.Surface` 背景，无边框高亮
 * - 选中中：金色 2dp 边框（`MenaColors.Primary`），轻度缩放视觉高亮
 *
 * testTag：`gift_item_{giftId}`
 *
 * @param gift       礼物数据
 * @param isSelected 是否被选中（控制金色边框）
 * @param onClick    点击回调，传出 [GiftVO.id]
 * @param modifier   可选 Modifier
 */
@Composable
fun GiftCard(
    gift: GiftVO,
    isSelected: Boolean,
    onClick: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(8.dp)
    val borderColor = if (isSelected) MenaColors.Primary else MenaColors.SurfaceVariant
    val borderWidth = if (isSelected) 2.dp else 1.dp

    Column(
        modifier = modifier
            .testTag("gift_item_${gift.id}")
            .clip(shape)
            .border(borderWidth, borderColor, shape)
            .background(MenaColors.Surface)
            .clickable { onClick(gift.id) }
            .padding(6.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // 礼物图标
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(1f),
            contentAlignment = Alignment.Center,
        ) {
            AsyncImage(
                model = gift.iconUrl,
                contentDescription = gift.name,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        Spacer(modifier = Modifier.height(4.dp))

        // 礼物名称
        Text(
            text = gift.name,
            style = MenaTypography.labelSmall,
            color = MenaColors.OnBackground,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth(),
        )

        // 价格
        Text(
            text = "💎 ${gift.price}",
            style = MenaTypography.labelSmall,
            color = MenaColors.Primary,
            maxLines = 1,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
