package com.voice.room.android.feature.gift.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography

/**
 * 礼物面板顶部余额条 (T-30028)
 *
 * 布局：Row —— 💎 余额数字（左） + 弹性空隙 + "充值"按钮（右）
 *
 * - 余额数字：`MenaColors.Primary` 金色
 * - 充值按钮：点击触发 [onRechargeClick]（T-30032 前: Toast；T-30032 后: InsufficientBalanceDialog）
 *
 * testTag：`gift_balance_bar`
 *
 * @param balance         当前钻石余额（Long）
 * @param onRechargeClick 充值按钮点击回调
 * @param modifier        可选 Modifier
 */
@Composable
fun BalanceBar(
    balance: Long,
    onRechargeClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .testTag("gift_balance_bar")
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "💎",
            style = MenaTypography.bodyMedium,
        )

        Spacer(modifier = Modifier.width(4.dp))

        Text(
            text = balance.toString(),
            style = MenaTypography.bodyMedium,
            color = MenaColors.Primary,
        )

        Spacer(modifier = Modifier.weight(1f))

        TextButton(onClick = onRechargeClick) {
            Text(
                text = "充值",
                style = MenaTypography.labelMedium,
                color = MenaColors.Primary,
            )
        }
    }
}
