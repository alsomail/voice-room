package com.voice.room.android.feature.room.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.voice.room.android.data.model.RoomMember

/**
 * 观众席单行 UI 组件（T-30039）
 *
 * 展示成员头像、昵称和角色徽章（owner 👑 / admin 🛡️）。
 *
 * @param member      要展示的成员数据
 * @param onClick     点击整行的回调
 * @param modifier    外部修饰符
 */
@Composable
fun MemberRow(
    member: RoomMember,
    onClick: (RoomMember) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clickable { onClick(member) }
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .testTag("audience_item_${member.id}"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // 头像（Coil 异步加载，带圆形裁剪）
        AsyncImage(
            model = member.avatarUrl,
            contentDescription = member.nickname,
            modifier = Modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(Color.Gray.copy(alpha = 0.3f)),
        )

        Spacer(modifier = Modifier.width(12.dp))

        // 昵称 + 角色徽章
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = member.nickname,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )

                if (member.role != "member") {
                    Spacer(modifier = Modifier.width(4.dp))
                    RoleBadge(role = member.role)
                }
            }
        }

        // 上麦标记
        if (member.slot != null) {
            Text(
                text = "🎤",
                fontSize = 14.sp,
                modifier = Modifier.padding(start = 8.dp),
            )
        }
    }
}

/**
 * 角色徽章 Composable
 *
 * - owner → 👑
 * - admin → 🛡️
 * - 其他 → 不显示
 */
@Composable
private fun RoleBadge(role: String) {
    val (emoji, bgColor) = when (role) {
        "owner" -> "👑" to Color(0xFFFFD700).copy(alpha = 0.2f)
        "admin" -> "🛡️" to Color(0xFF4CAF50).copy(alpha = 0.2f)
        else -> return
    }
    Box(
        modifier = Modifier
            .background(bgColor, shape = RoundedCornerShape(4.dp))
            .padding(horizontal = 4.dp, vertical = 2.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(text = emoji, fontSize = 12.sp)
    }
}
