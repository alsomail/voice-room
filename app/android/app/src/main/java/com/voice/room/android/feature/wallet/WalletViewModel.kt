package com.voice.room.android.feature.wallet

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.paging.Pager
import androidx.paging.PagingConfig
import androidx.paging.PagingData
import androidx.paging.cachedIn
import com.google.gson.JsonParser
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.wallet.WalletTxnPagingSource
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.WalletTxn
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * WalletViewModel — 钱包页 ViewModel (T-30027)
 *
 * 职责：
 * - init 时调用 [loadBalance] 拉取余额
 * - 订阅 [IWebSocketClient.state] 监听 `BalanceUpdated` 实时事件，更新余额并触发流水刷新
 * - 暴露 [txnPagingFlow] 供 UI 层通过 Paging3 展示流水列表
 * - [refresh] 下拉刷新：重新拉取余额 + 发射 [WalletEvent.RefreshTransactions]
 * - [onRechargeClick] 发射 [WalletEvent.ShowToast("即将上线")]
 * - 401 错误 → 发射 [WalletEvent.NavigateToLogin]
 *
 * 结构化并发：[CancellationException] 必须 re-throw，不得吞噬。
 *
 * @param walletRepository 钱包 Repository（生产: RetrofitWalletRepository，测试: Fake）
 * @param wsClient         WebSocket 客户端（生产: OkHttpWebSocketClient，测试: FakeWebSocketClient）
 */
class WalletViewModel(
    private val walletRepository: IWalletRepository,
    private val wsClient: IWebSocketClient,
) : ViewModel() {

    // ─── State & Events ───────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow(WalletUiState())
    val uiState: StateFlow<WalletUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<WalletEvent>()
    val events: SharedFlow<WalletEvent> = _events.asSharedFlow()

    // ─── Paging Flow ──────────────────────────────────────────────────────────

    /**
     * 流水 Paging3 数据流，已通过 [cachedIn] 缓存到 [viewModelScope]。
     *
     * UI 层使用 `collectAsLazyPagingItems()` 消费。
     * 调用 `lazyPagingItems.refresh()` 可刷新列表。
     */
    val txnPagingFlow: Flow<PagingData<WalletTxn>> = Pager(
        config = PagingConfig(
            pageSize = 20,
            initialLoadSize = 20,
            enablePlaceholders = false,
            prefetchDistance = 5,
        ),
        pagingSourceFactory = { WalletTxnPagingSource(walletRepository) },
    ).flow.cachedIn(viewModelScope)

    // ─── Init ─────────────────────────────────────────────────────────────────

    init {
        loadBalance()
        subscribeToWsEvents()
    }

    // ─── Public Actions ───────────────────────────────────────────────────────

    /**
     * 拉取余额。
     *
     * 成功 → [WalletUiState.balance] 更新，[WalletUiState.loadingBalance] = false
     * 401 → 发射 [WalletEvent.NavigateToLogin]
     * 其他异常 → [WalletUiState.error] 设置
     */
    fun loadBalance() {
        viewModelScope.launch {
            _uiState.update { it.copy(loadingBalance = true, error = null) }
            walletRepository.getBalance()
                .onSuccess { balance ->
                    _uiState.update { it.copy(balance = balance, loadingBalance = false) }
                }
                .onFailure { e ->
                    if (e is CancellationException) throw e
                    if (e is ApiException && e.code == 401) {
                        _uiState.update { it.copy(loadingBalance = false) }
                        _events.emit(WalletEvent.NavigateToLogin)
                    } else {
                        _uiState.update { it.copy(loadingBalance = false, error = e.message) }
                    }
                }
        }
    }

    /**
     * 下拉刷新：重新拉取余额 + 通知 UI 刷新流水。
     *
     * 401 错误与 [loadBalance] 保持一致，发射 [WalletEvent.NavigateToLogin]。
     */
    fun refresh() {
        viewModelScope.launch {
            _uiState.update { it.copy(refreshing = true, error = null) }
            walletRepository.getBalance()
                .onSuccess { balance ->
                    _uiState.update { it.copy(balance = balance, refreshing = false) }
                    _events.emit(WalletEvent.RefreshTransactions)
                }
                .onFailure { e ->
                    if (e is CancellationException) throw e
                    if (e is ApiException && e.code == 401) {
                        _uiState.update { it.copy(refreshing = false) }
                        _events.emit(WalletEvent.NavigateToLogin)
                    } else {
                        _uiState.update { it.copy(refreshing = false, error = e.message) }
                    }
                }
        }
    }

    /**
     * 充值按钮点击：显示"即将上线"Toast（W27-03）。
     */
    fun onRechargeClick() {
        viewModelScope.launch {
            _events.emit(WalletEvent.ShowToast("即将上线"))
        }
    }

    // ─── WebSocket Subscription ───────────────────────────────────────────────

    /**
     * 订阅 WS 消息流，处理 `BalanceUpdated` 事件。
     *
     * 消息格式（§6.4.1 协议）：
     * ```json
     * {"type":"BalanceUpdated","msg_id":"uuid","payload":{"diamond_balance":4800,...},"timestamp":...}
     * ```
     * 成功解析后：更新 [WalletUiState.balance] + 发射 [WalletEvent.RefreshTransactions]
     * 解析失败静默忽略，不影响 UI 状态。
     */
    private fun subscribeToWsEvents() {
        viewModelScope.launch {
            wsClient.state.collect { state ->
                if (state is WebSocketState.Message) {
                    handleWsMessage(state.text)
                }
            }
        }
    }

    private fun handleWsMessage(text: String) {
        try {
            val json = JsonParser.parseString(text)?.asJsonObject ?: return
            if (json.get("type")?.asString == "BalanceUpdated") {
                // 按协议 §6.4.1 读取 payload.diamond_balance
                val payload = json.getAsJsonObject("payload") ?: return
                val newBalance = payload.get("diamond_balance")?.asLong ?: return
                _uiState.update { it.copy(balance = newBalance) }
                viewModelScope.launch {
                    _events.emit(WalletEvent.RefreshTransactions)
                }
            }
        } catch (e: Exception) {
            // 忽略格式错误的 WS 消息，不影响 UI 稳定性
        }
    }

    // ─── Factory ──────────────────────────────────────────────────────────────

    companion object {
        /**
         * 工厂方法，供 `viewModel(factory = ...)` 注入依赖使用。
         */
        fun factory(
            walletRepository: IWalletRepository,
            wsClient: IWebSocketClient,
        ) = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T =
                WalletViewModel(walletRepository, wsClient) as T
        }
    }
}
