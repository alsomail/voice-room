package com.voice.room.android.feature.room.components

import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.room.governance.Role

/**
 * 角色徽章组件（T-30043 AN43-07）
 *
 * 根据用户角色显示对应图标：
 * - [Role.Owner] → 👑 金色皇冠
 * - [Role.Admin] → 🛡️ 金色盾牌
 * - [Role.Member] → 无显示
 *
 * testTag: `role_badge_{userId}`（userId 由调用方传入）
 *
 * @param role     用户角色
 * @param userId   用户 ID（用于 testTag）
 * @param modifier Modifier
 */
@Composable
fun RoleBadge(
    role: Role,
    userId: String = "",
    modifier: Modifier = Modifier,
) {
    when (role) {
        Role.Owner -> Text(
            text = "👑",
            color = MenaColors.Primary,
            modifier = modifier.testTag("role_badge_$userId"),
        )
        Role.Admin -> Text(
            text = "🛡️",
            color = MenaColors.Primary,
            modifier = modifier.testTag("role_badge_$userId"),
        )
        Role.Member -> Unit // 普通成员无徽章
    }
}
