package com.voice.room.android.feature.wallet

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * 余额不足引导弹窗 (T-30032)
 *
 * 当服务端返回 40290 或本地判断 balance < totalPrice 时触发，引导用户进入钱包充值页面。
 *
 * - 标题："钻石不足"
 * - 正文：当前余额 / 所需 / 差额（三行）
 * - 按钮：[取消] [去充值 GoldButton]
 * - `dismissOnClickOutside = false`（点外部不关闭，防止误操作）
 * - `dismissOnBackPress = true`（返回键可关闭）
 *
 * testTag 协议（与 Review 规格对齐）：
 *   - 弹窗容器：`dialog_insufficient_balance`
 *   - 去充值按钮：`btn_go_to_wallet`
 *   - 取消按钮：`btn_insufficient_cancel`
 *
 * @param currentBalance 当前用户钻石余额（来自 GiftPanelUiState.balance）
 * @param required       所需钻石总数（来自 GiftPanelUiState.totalPrice）
 * @param onGoToWallet   点击"去充值"回调 → 调用 GiftPanelViewModel.onGoToWallet()
 * @param onDismiss      点击"取消"或返回键回调 → 调用 GiftPanelViewModel.dismissInsufficientDialog()
 */
@Composable
fun InsufficientBalanceDialog(
    currentBalance: Long,
    required: Long,
    onGoToWallet: () -> Unit,
    onDismiss: () -> Unit,
) {
    val deficit = (required - currentBalance).coerceAtLeast(0L)

    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = Modifier.testTag("dialog_insufficient_balance"),
        properties = androidx.compose.ui.window.DialogProperties(
            dismissOnClickOutside = false,
            dismissOnBackPress = true,
        ),
        title = {
            Text(
                text = "钻石不足",
                style = MenaTypography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = MenaColors.OnBackground,
            )
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp),
            ) {
                Text(
                    text = "当前余额：💎 $currentBalance",
                    style = MenaTypography.bodyMedium,
                    color = MenaColors.OnBackground,
                )
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = "所需：💎 $required",
                    style = MenaTypography.bodyMedium,
                    color = MenaColors.OnBackground,
                )
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = "差：💎 $deficit",
                    style = MenaTypography.bodyMedium,
                    color = MenaColors.Error,
                )
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.testTag("btn_insufficient_cancel"),
            ) {
                Text(
                    text = "取消",
                    color = MenaColors.OnBackgroundTertiary,
                )
            }
        },
        confirmButton = {
            GoldButton(
                text = "去充值",
                onClick = onGoToWallet,
                modifier = Modifier.testTag("btn_go_to_wallet"),
            )
        },
        containerColor = MenaColors.Surface,
    )
}
