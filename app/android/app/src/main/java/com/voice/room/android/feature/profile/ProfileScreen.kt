package com.voice.room.android.feature.profile

import android.widget.Toast
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.MenaColors

/**
 * ProfileScreen — 个人中心 Stateful Composable (T-30024)
 *
 * 职责：
 * - 持有 [ProfileViewModel]，监听 [ProfileEvent] 事件流
 * - 将 UI 状态传递给无状态的 [ProfileContent]
 * - 控制退出登录确认弹框的显隐
 * - 处理剪贴板写入（ClipboardManager）
 * - T-30027：接受 [onNavigateToWallet] 回调，转发给 [ProfileContent] 余额行点击
 *
 * @param appContainer       依赖容器，提供 userRepository / tokenManager
 * @param onLogout           退出登录成功后导航回 LoginScreen 的回调
 * @param onNavigateToWallet 点击余额行时导航到钱包页（T-30027）
 * @param modifier           外部 Modifier
 */
@Composable
fun ProfileScreen(
    appContainer: AppContainer,
    onLogout: () -> Unit,
    onNavigateToWallet: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val viewModel: ProfileViewModel = viewModel(
        factory = ProfileViewModel.factory(
            userRepository = appContainer.userRepository,
            tokenManager = appContainer.tokenManager,
        )
    )
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    var showLogoutDialog by remember { mutableStateOf(false) }

    // ── 事件监听 ────────────────────────────────────────────────────────────
    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                is ProfileEvent.NavigateToLogin -> onLogout()
                is ProfileEvent.ShowToast ->
                    Toast.makeText(context, event.message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    // ── 主内容 ──────────────────────────────────────────────────────────────
    ProfileContent(
        uiState = uiState,
        modifier = modifier,
        onCopyId = { userId ->
            clipboardManager.setText(AnnotatedString(userId))
            viewModel.copyId(userId)
        },
        onLogoutClick = { showLogoutDialog = true },
        onRetry = { viewModel.loadProfile() },
        onNavigateToWallet = onNavigateToWallet,
    )

    // ── 退出登录二次确认弹框 ─────────────────────────────────────────────────
    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            modifier = Modifier.testTag("logout_confirm_dialog"),
            shape = RoundedCornerShape(16.dp),
            containerColor = MenaColors.Surface,
            title = {
                Text(
                    text = "退出登录",
                    color = MenaColors.OnBackground,
                )
            },
            text = {
                Text(
                    text = "确认退出当前账号？",
                    color = MenaColors.OnBackgroundSecondary,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showLogoutDialog = false
                        viewModel.logout()
                    },
                    modifier = Modifier.testTag("logout_confirm_button"),
                ) {
                    Text(text = "确认", color = MenaColors.Error)
                }
            },
            dismissButton = {
                TextButton(
                    onClick = { showLogoutDialog = false },
                    modifier = Modifier.testTag("logout_cancel_button"),
                ) {
                    Text(text = "取消", color = MenaColors.OnBackgroundSecondary)
                }
            },
        )
    }
}
