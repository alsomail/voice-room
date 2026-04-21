package com.voice.room.android.feature.gift

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography
import com.voice.room.android.feature.gift.components.BalanceBar
import com.voice.room.android.feature.gift.components.CountSelector
import com.voice.room.android.feature.gift.components.GiftCard
import kotlinx.coroutines.flow.SharedFlow

/**
 * 礼物面板 BottomSheet (T-30028)
 *
 * - 弹出条件：先检查连接 & 房间状态，由 RoomScreen 控制 visible
 * - 高度：55%（`sheetMaxWidth`）
 * - 顶部：[BalanceBar]（余额 + 充值）+ × 关闭按钮
 * - Tab Row：Hot / All / Backpack（Phase 2 占位禁用）
 * - 礼物网格：[LazyVerticalGrid]（4 列）
 * - 数量选择器：[CountSelector]
 * - 接收者槽：T-30029 接入，占位 Row
 * - 送出按钮：T-30030 接入，当前根据 canSend 控制 enabled
 *
 * testTag 协议（与 TDS §testTag 对齐）：
 *   - 整体容器：  `gift_panel_sheet`
 *   - 余额条：    `gift_balance_bar`（由 BalanceBar 内部设置）
 *   - 关闭按钮：  `btn_gift_close`
 *   - 礼物卡片：  `gift_item_{giftId}`（由 GiftCard 内部设置）
 *   - 数量档位：  `count_option_{value}`（由 CountSelector 内部设置）
 *   - 送出按钮：  `btn_send_gift`
 *   - 接收者槽：  `recipient_selector`
 *
 * @param uiState      礼物面板 UI 状态
 * @param events       ViewModel 一次性事件流
 * @param onDismiss    关闭回调（外部点击 / × / 返回键）
 * @param onSelectGift 选中礼物回调
 * @param onSelectCount 数量档位选择回调
 * @param onSendGift   送出按钮点击回调（T-30030 接入）
 * @param onRechargeClick 充值按钮回调
 * @param modifier     可选 Modifier
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GiftPanelBottomSheet(
    uiState: GiftPanelUiState,
    events: SharedFlow<GiftPanelEvent>,
    onDismiss: () -> Unit,
    onSelectGift: (String) -> Unit,
    onSelectCount: (Int) -> Unit,
    onSelectTab: (GiftTab) -> Unit,
    onSendGift: () -> Unit = {},
    onRechargeClick: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    // 消费 ViewModel 一次性事件
    LaunchedEffect(Unit) {
        events.collect { event ->
            when (event) {
                is GiftPanelEvent.ShowRechargeHint ->
                    Toast.makeText(context, "充值功能即将上线", Toast.LENGTH_SHORT).show()
                is GiftPanelEvent.ShowToast ->
                    Toast.makeText(context, event.message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = modifier.testTag("gift_panel_sheet"),
        containerColor = MenaColors.Surface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MenaColors.Surface),
        ) {
            // ── 顶部：余额条 + 关闭按钮 ──────────────────────────────────────
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                BalanceBar(
                    balance = uiState.balance,
                    onRechargeClick = onRechargeClick,
                    modifier = Modifier.weight(1f),
                )

                IconButton(
                    onClick = onDismiss,
                    modifier = Modifier.testTag("btn_gift_close"),
                ) {
                    Icon(
                        imageVector = Icons.Default.Close,
                        contentDescription = "关闭礼物面板",
                        tint = MenaColors.OnBackground,
                    )
                }
            }

            // ── Tab Row: Hot / All / Backpack ─────────────────────────────────
            val tabs = listOf(GiftTab.Hot, GiftTab.All, GiftTab.Backpack)
            val tabLabels = listOf("热门", "全部", "背包")
            TabRow(
                selectedTabIndex = tabs.indexOf(uiState.activeTab),
                containerColor = MenaColors.Surface,
                contentColor = MenaColors.Primary,
            ) {
                tabs.forEachIndexed { index, tab ->
                    Tab(
                        selected = uiState.activeTab == tab,
                        onClick = { onSelectTab(tab) },
                        enabled = tab != GiftTab.Backpack, // Backpack 占位禁用
                        text = {
                            Text(
                                text = tabLabels[index],
                                style = MenaTypography.labelMedium,
                                color = if (uiState.activeTab == tab) MenaColors.Primary
                                else MenaColors.OnBackgroundTertiary,
                            )
                        },
                    )
                }
            }

            // ── 礼物网格 / 骨架屏 / 错误状态 ────────────────────────────────
            when {
                uiState.loading -> {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(32.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator(color = MenaColors.Primary)
                    }
                }
                uiState.error != null -> {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Text(
                            text = uiState.error,
                            style = MenaTypography.bodyMedium,
                            color = MenaColors.Error,
                        )
                        TextButton(onClick = { /* retryLoad 由 onSelectTab 逻辑触发，外部通过 onRetry 传入 */ }) {
                            Text("点击重试", color = MenaColors.Primary)
                        }
                    }
                }
                else -> {
                    LazyVerticalGrid(
                        columns = GridCells.Fixed(4),
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(bottom = 4.dp),
                    ) {
                        items(uiState.displayGifts, key = { it.id }) { gift ->
                            GiftCard(
                                gift = gift,
                                isSelected = gift.id == uiState.selectedGiftId,
                                onClick = onSelectGift,
                            )
                        }
                    }
                }
            }

            // ── 数量选择器 ────────────────────────────────────────────────────
            CountSelector(
                selectedCount = uiState.selectedCount,
                onCountSelected = onSelectCount,
            )

            // ── 接收者槽（T-30029 占位） ──────────────────────────────────────
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("recipient_selector")
                    .padding(horizontal = 16.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "送给：",
                    style = MenaTypography.labelMedium,
                    color = MenaColors.OnBackground,
                )
                Text(
                    text = uiState.recipients.firstOrNull { it.userId == uiState.selectedRecipientId }
                        ?.nickname ?: "请选择接收者",
                    style = MenaTypography.bodyMedium,
                    color = if (uiState.selectedRecipientId != null) MenaColors.Primary
                    else MenaColors.OnBackgroundTertiary,
                )
            }

            // ── 送出按钮 ──────────────────────────────────────────────────────
            val sendButtonText = when {
                uiState.recipients.isEmpty()   -> "当前无人在麦"
                uiState.isBalanceInsufficient  -> "余额不足"
                uiState.selectedGift != null   -> "送出 ${uiState.totalPrice} 💎"
                else                           -> "选择礼物"
            }

            Button(
                onClick = onSendGift,
                enabled = uiState.canSend,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("btn_send_gift")
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MenaColors.Primary,
                    disabledContainerColor = MenaColors.SurfaceVariant,
                ),
            ) {
                Text(
                    text = sendButtonText,
                    style = MenaTypography.labelLarge,
                    color = if (uiState.canSend) MenaColors.Background
                    else MenaColors.OnBackgroundTertiary,
                )
            }
        }
    }
}
