package com.voice.room.android.feature.room

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.voice.room.android.common.ui.OnlineCountBadge
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography
import com.voice.room.android.domain.room.RoomItem

/**
 * RoomCard — 深色房间卡片 (T-30022 视觉升级)
 *
 * 布局结构:
 * - 外层: Card(shape=RoundedCornerShape(16.dp), containerColor=MenaColors.Surface)
 * - 上半区: Box(height=120.dp, background=SurfaceVariant) 渐变色块占位区
 *   - 密码房: 右上角 Lock 图标覆盖层
 * - 下半区: Column(padding=12.dp)
 *   - 标题(bodyMedium, OnBackground, maxLines=2, ellipsis)
 *   - Row(房主名 labelSmall/OnBackgroundSecondary + OnlineCountBadge)
 *
 * @param room    RoomItem 领域模型
 * @param onClick 点击回调（触发进房导航）
 * @param modifier 可选修饰符
 */
@Composable
fun RoomCard(
    room: RoomItem,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(
        onClick = onClick,
        modifier = modifier
            .fillMaxWidth()
            .testTag("room_card_${room.roomId}"),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MenaColors.Surface,
        ),
    ) {
        Column {
            // ── 上半区：渐变色块占位 + 密码锁图标 ──────────
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(120.dp)
                    .background(MenaColors.SurfaceVariant),
            ) {
                // 密码房: 右上角 Lock 图标
                if (room.roomType == "password") {
                    Icon(
                        imageVector = Icons.Default.Lock,
                        contentDescription = null,
                        tint = MenaColors.OnBackground,
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(8.dp)
                            .size(20.dp)
                            .testTag("room_type_icon_password"),
                    )
                }
            }

            // ── 下半区：标题 + 房主名 + 在线人数 ──────────
            Column(
                modifier = Modifier.padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                // 标题：白色，最多 2 行 + ellipsis
                Text(
                    text = room.title,
                    style = MenaTypography.bodyMedium,
                    color = MenaColors.OnBackground,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )

                // Row: 房主昵称 + OnlineCountBadge
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = room.ownerNickname,
                        style = MenaTypography.labelSmall,
                        color = MenaColors.OnBackgroundSecondary,
                    )
                    OnlineCountBadge(count = room.memberCount)
                }
            }
        }
    }
}
