package com.voice.room.android.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Divider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.core.theme.AvatarWithFrame
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.domain.user.UserProfile

/**
 * ProfileContent — 个人中心 Stateless UI Composable (T-30024)
 *
 * 根据 [ProfileUiState] 展示不同内容：
 * - [ProfileUiState.Loading]  → 居中加载指示器（testTag: profile_loading）
 * - [ProfileUiState.Success]  → 完整的个人信息卡片
 * - [ProfileUiState.Error]    → 错误提示 + 重试按钮（testTag: profile_error）
 *
 * testTag 协议（对应 TDS §详细设计 ProfileContent）：
 * - profile_screen / profile_avatar / profile_nickname / profile_id_row / profile_id_text
 * - profile_balance / profile_cache_badge / profile_settings_item
 * - profile_logout_button / profile_error / profile_retry_button / profile_loading
 *
 * @param uiState        当前 UI 状态
 * @param onCopyId       点击 ID 区域时回调，参数为用户 ID
 * @param onLogoutClick  点击"退出登录"按钮时回调
 * @param onRetry        点击"重试"按钮时回调（Error 状态）
 * @param modifier       外部 Modifier
 */
@Composable
fun ProfileContent(
    uiState: ProfileUiState,
    onCopyId: (String) -> Unit,
    onLogoutClick: () -> Unit,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MenaColors.Background)
            .testTag("profile_screen"),
        contentAlignment = Alignment.TopCenter,
    ) {
        when (uiState) {
            is ProfileUiState.Loading -> ProfileLoadingContent()
            is ProfileUiState.Success -> ProfileSuccessContent(
                profile = uiState.profile,
                fromCache = uiState.fromCache,
                onCopyId = onCopyId,
                onLogoutClick = onLogoutClick,
            )
            is ProfileUiState.Error -> ProfileErrorContent(
                message = uiState.message,
                onRetry = onRetry,
            )
        }
    }
}

// ─── Loading ───────────────────────────────────────────────────────────────────

@Composable
private fun ProfileLoadingContent() {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .testTag("profile_loading"),
        contentAlignment = Alignment.Center,
    ) {
        CircularProgressIndicator(color = MenaColors.Primary)
    }
}

// ─── Success ──────────────────────────────────────────────────────────────────

@Composable
private fun ProfileSuccessContent(
    profile: UserProfile,
    fromCache: Boolean,
    onCopyId: (String) -> Unit,
    onLogoutClick: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Spacer(modifier = Modifier.height(40.dp))

        // ── 头像 ─────────────────────────────────────────
        AvatarWithFrame(
            imageUrl = profile.avatar,
            size = 80.dp,
            showFrame = true,
            modifier = Modifier.testTag("profile_avatar"),
        )

        Spacer(modifier = Modifier.height(16.dp))

        // ── 昵称 ─────────────────────────────────────────
        Text(
            text = profile.nickname,
            style = MaterialTheme.typography.titleLarge,
            color = MenaColors.OnBackground,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.testTag("profile_nickname"),
        )

        Spacer(modifier = Modifier.height(8.dp))

        // ── 用户 ID 行（含复制图标）──────────────────────
        Row(
            modifier = Modifier
                .testTag("profile_id_row")
                .clickable { onCopyId(profile.id) }
                .padding(vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
        ) {
            Text(
                text = "ID: ${profile.id}",
                style = MaterialTheme.typography.bodyMedium,
                color = MenaColors.OnBackgroundSecondary,
                modifier = Modifier.testTag("profile_id_text"),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Icon(
                imageVector = Icons.Default.ContentCopy,
                contentDescription = "复制 ID",
                tint = MenaColors.OnBackgroundSecondary,
                modifier = Modifier.size(16.dp),
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        // ── 余额 ─────────────────────────────────────────
        Text(
            text = "💰 ${profile.coinBalance} 金币",
            style = MaterialTheme.typography.bodyLarge,
            color = MenaColors.Primary,
            modifier = Modifier.testTag("profile_balance"),
        )

        // ── 缓存标记（仅 fromCache=true 时显示）─────────
        if (fromCache) {
            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = "缓存",
                style = MaterialTheme.typography.labelSmall,
                color = MenaColors.OnBackgroundTertiary,
                fontSize = 10.sp,
                modifier = Modifier
                    .background(
                        color = MenaColors.SurfaceVariant,
                        shape = androidx.compose.foundation.shape.RoundedCornerShape(4.dp),
                    )
                    .padding(horizontal = 6.dp, vertical = 2.dp)
                    .testTag("profile_cache_badge"),
            )
        }

        Spacer(modifier = Modifier.height(32.dp))
        Divider(color = MenaColors.SurfaceVariant)
        Spacer(modifier = Modifier.height(16.dp))

        // ── 设置入口（占位，后续扩展）──────────────────
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .testTag("profile_settings_item")
                .padding(vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = Icons.Default.Settings,
                contentDescription = "设置",
                tint = MenaColors.OnBackgroundSecondary,
            )
            Spacer(modifier = Modifier.width(12.dp))
            Text(
                text = "设置",
                style = MaterialTheme.typography.bodyLarge,
                color = MenaColors.OnBackgroundSecondary,
            )
        }

        Divider(color = MenaColors.SurfaceVariant)
        Spacer(modifier = Modifier.height(40.dp))

        // ── 退出登录按钮 ─────────────────────────────────
        TextButton(
            onClick = onLogoutClick,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("profile_logout_button"),
        ) {
            Text(
                text = "退出登录",
                color = MenaColors.Error,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Medium,
            )
        }
    }
}

// ─── Error ────────────────────────────────────────────────────────────────────

@Composable
private fun ProfileErrorContent(
    message: String,
    onRetry: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .testTag("profile_error"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MenaColors.OnBackgroundSecondary,
        )
        Spacer(modifier = Modifier.height(16.dp))
        GoldButton(
            text = "重试",
            onClick = onRetry,
            modifier = Modifier.testTag("profile_retry_button"),
        )
    }
}
