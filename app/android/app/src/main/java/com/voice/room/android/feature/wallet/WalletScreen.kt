package com.voice.room.android.feature.wallet

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.paging.LoadState
import androidx.paging.compose.LazyPagingItems
import androidx.paging.compose.collectAsLazyPagingItems
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.domain.wallet.WalletTxn

// ─── TDS testTag 协议 ──────────────────────────────────────────────────────────
// Key('wallet_balance_value')  → Text 余额数字
// Key('btn_wallet_recharge')   → 充值按钮
// Key('wallet_txn_list')       → 流水 LazyColumn
// Key('wallet_empty')          → 空状态占位

/**
 * WalletScreen — 钱包页 Stateful Composable (T-30027)
 *
 * 职责：
 * - 持有 [WalletViewModel]，监听 [WalletEvent] 事件流
 * - 展示余额大卡片（💎 + 充值按钮）
 * - 展示流水分页列表（Paging3，下拉刷新）
 * - WS `BalanceUpdated` 实时更新余额
 * - 401 错误跳转 LoginScreen（复用 T-30003 拦截器）
 *
 * @param appContainer     依赖容器
 * @param onNavigateBack   返回上级页面回调
 * @param onNavigateToLogin 跳转登录页回调（401 处理）
 * @param modifier         外部 Modifier
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletScreen(
    appContainer: AppContainer,
    onNavigateBack: () -> Unit,
    onNavigateToLogin: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val viewModel: WalletViewModel = viewModel(
        factory = WalletViewModel.factory(
            walletRepository = appContainer.walletRepository,
            wsClient = appContainer.webSocketClient,
        )
    )
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val txnPagingItems = viewModel.txnPagingFlow.collectAsLazyPagingItems()
    val context = LocalContext.current

    // ── 事件监听 ────────────────────────────────────────────────────────────
    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                is WalletEvent.ShowToast ->
                    Toast.makeText(context, event.message, Toast.LENGTH_SHORT).show()

                is WalletEvent.NavigateToLogin -> onNavigateToLogin()

                is WalletEvent.RefreshTransactions -> txnPagingItems.refresh()
            }
        }
    }

    Scaffold(
        modifier = modifier,
        containerColor = MenaColors.Background,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = "我的钱包",
                        style = MaterialTheme.typography.titleMedium,
                        color = MenaColors.OnBackground,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "返回",
                            tint = MenaColors.OnBackground,
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MenaColors.Background,
                ),
            )
        },
    ) { innerPadding ->
        PullToRefreshBox(
            isRefreshing = uiState.refreshing,
            onRefresh = { viewModel.refresh() },
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .testTag("wallet_txn_list"),
                contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                // ── 余额大卡片 ──────────────────────────────────────────────
                item {
                    WalletBalanceCard(
                        balance = uiState.balance,
                        loading = uiState.loadingBalance,
                        onRechargeClick = { viewModel.onRechargeClick() },
                    )
                }

                // ── 流水列表 ────────────────────────────────────────────────
                if (txnPagingItems.itemCount == 0 &&
                    txnPagingItems.loadState.refresh !is LoadState.Loading
                ) {
                    // 空状态
                    item {
                        WalletEmptyState()
                    }
                } else {
                    items(
                        count = txnPagingItems.itemCount,
                        key = { index -> txnPagingItems[index]?.id ?: "txn_$index" },
                    ) { index ->
                        val txn = txnPagingItems[index]
                        if (txn != null) {
                            WalletTxnItem(txn = txn)
                        }
                    }

                    // 加载更多指示器
                    if (txnPagingItems.loadState.append is LoadState.Loading) {
                        item {
                            Box(
                                modifier = Modifier.fillMaxWidth(),
                                contentAlignment = Alignment.Center,
                            ) {
                                CircularProgressIndicator(
                                    modifier = Modifier
                                        .size(24.dp)
                                        .padding(vertical = 8.dp),
                                    color = MenaColors.Primary,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─── 余额大卡片 ────────────────────────────────────────────────────────────────

@Composable
private fun WalletBalanceCard(
    balance: Long,
    loading: Boolean,
    onRechargeClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MenaColors.Surface),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = "钻石余额",
                style = MaterialTheme.typography.bodyMedium,
                color = MenaColors.OnBackgroundSecondary,
            )
            Spacer(modifier = Modifier.height(12.dp))

            if (loading) {
                CircularProgressIndicator(
                    modifier = Modifier.size(40.dp),
                    color = MenaColors.Primary,
                )
            } else {
                Text(
                    text = "💎 $balance",
                    style = MaterialTheme.typography.headlineLarge,
                    color = MenaColors.Primary,
                    fontWeight = FontWeight.Bold,
                    fontSize = 40.sp,
                    modifier = Modifier.testTag("wallet_balance_value"),
                )
            }

            Spacer(modifier = Modifier.height(20.dp))

            GoldButton(
                text = "充值",
                onClick = onRechargeClick,
                modifier = Modifier.testTag("btn_wallet_recharge"),
            )
        }
    }
}

// ─── 流水列表项 ────────────────────────────────────────────────────────────────

@Composable
private fun WalletTxnItem(txn: WalletTxn) {
    val isIncome = txn.amount > 0
    val amountColor = if (isIncome) Color(0xFF4CAF50) else MenaColors.Error
    val amountText = if (isIncome) "+${txn.amount}" else "${txn.amount}"

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = MenaColors.Surface),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = txn.reason,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MenaColors.OnBackground,
                    fontWeight = FontWeight.Medium,
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = txn.createdAt.take(10), // 仅展示日期部分
                    style = MaterialTheme.typography.bodySmall,
                    color = MenaColors.OnBackgroundSecondary,
                )
            }
            Spacer(modifier = Modifier.width(12.dp))
            Text(
                text = amountText,
                style = MaterialTheme.typography.titleMedium,
                color = amountColor,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

// ─── 空状态 ────────────────────────────────────────────────────────────────────

@Composable
private fun WalletEmptyState() {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(200.dp)
            .testTag("wallet_empty"),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = "💫",
                fontSize = 48.sp,
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "暂无流水",
                style = MaterialTheme.typography.bodyLarge,
                color = MenaColors.OnBackgroundSecondary,
            )
        }
    }
}
