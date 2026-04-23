package com.voice.room.android.feature.room.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.voice.room.android.data.model.RoomMember
import com.voice.room.android.feature.room.governance.UserAction

/**
 * 用户操作菜单 BottomSheet（T-30040）
 *
 * 根据 [actions] 列表动态渲染操作菜单项。
 * 操作项列表由 ViewModel 调用 [computeActions] 计算后传入。
 *
 * ### testTag
 * - 整体 Sheet：`user_action_sheet`
 * - 每个操作项：`user_action_${action.name}`（如 `user_action_AssignAdmin`）
 *
 * @param member      目标成员信息（显示在 Sheet 顶部）
 * @param actions     当前可用操作列表（由 [computeActions] 计算）
 * @param onAction    操作项点击回调
 * @param onDismiss   关闭 Sheet 的回调
 * @param modifier    外部修饰符
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UserActionBottomSheet(
    member: RoomMember,
    actions: List<UserAction>,
    onAction: (UserAction) -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = modifier.testTag("user_action_sheet"),
    ) {
        Column(modifier = Modifier.fillMaxWidth()) {
            // ── 顶部：目标用户信息 ──────────────────────────────────────────────
            UserInfoHeader(member = member)

            HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))

            // ── 操作列表（空则不显示任何内容，Sheet 依然弹出但内容为空） ──────────
            if (actions.isEmpty()) {
                Spacer(modifier = Modifier.height(32.dp))
            } else {
                LazyColumn {
                    items(actions, key = { it.name }) { action ->
                        ActionItem(
                            action = action,
                            onClick = {
                                onAction(action)
                            },
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
            }
        }
    }
}

/**
 * 目标用户信息头部（头像 + 昵称 + 角色）
 */
@Composable
private fun UserInfoHeader(member: RoomMember) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        AsyncImage(
            model = member.avatarUrl,
            contentDescription = member.nickname,
            modifier = Modifier
                .size(48.dp)
                .clip(CircleShape),
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = member.nickname,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )
            if (member.role != "member") {
                Text(
                    text = when (member.role) {
                        "owner" -> "👑 房主"
                        "admin" -> "🛡️ 管理员"
                        else -> member.role
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * 单个操作菜单项
 */
@Composable
private fun ActionItem(
    action: UserAction,
    onClick: () -> Unit,
) {
    Text(
        text = action.toDisplayName(),
        style = MaterialTheme.typography.bodyLarge,
        fontSize = 16.sp,
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 20.dp, vertical = 14.dp)
            .testTag("user_action_${action.name}"),
        color = if (action == UserAction.Kick) MaterialTheme.colorScheme.error
        else MaterialTheme.colorScheme.onSurface,
    )
}

/**
 * [UserAction] 枚举转换为 UI 显示文字
 */
private fun UserAction.toDisplayName(): String = when (this) {
    UserAction.AssignAdmin   -> "任命管理员"
    UserAction.RevokeAdmin   -> "卸任管理员"
    UserAction.ForceTakeMic  -> "抱上麦"
    UserAction.ForceLeaveMic -> "抱下麦"
    UserAction.MuteMic       -> "禁麦"
    UserAction.MuteChat      -> "禁言"
    UserAction.Kick          -> "踢出房间"
    UserAction.ViewProfile   -> "查看资料"
    UserAction.Report        -> "举报"
}
