package com.voice.room.android.feature.gift

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.google.gson.JsonParser
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.gift.MicUserVO
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * 礼物面板 ViewModel (T-30028)
 *
 * 职责：
 * - [loadGifts] 拉取礼物列表（打开面板时调用，内存缓存 60s 由 Repository 处理）
 * - 订阅 [IWebSocketClient.state] 处理 `BalanceUpdated` 实时余额更新
 * - [selectGift] / [selectCount] / [selectRecipient] 维护选中态
 * - [updateRecipients] 接收来自 RoomViewModel 的当前在麦用户列表，自动选中第一个
 * - [dismiss] 关闭面板时清除选中态
 * - [retryLoad] 网络失败后点击重试
 * - [selectTab] 切换 Hot/All Tab
 * - [onRechargeClick] 充值按钮点击（发射 [GiftPanelEvent.ShowRechargeHint]）
 *
 * ### 结构化并发
 * [CancellationException] 必须 re-throw，不得吞噬。
 *
 * @param giftRepository 礼物仓库（生产: RetrofitGiftRepository，测试: Fake）
 * @param wsClient       WebSocket 客户端（生产: OkHttpWebSocketClient，测试: FakeWebSocketClient）
 */
class GiftPanelViewModel(
    private val giftRepository: IGiftRepository,
    private val wsClient: IWebSocketClient,
) : ViewModel() {

    // ─── State & Events ───────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow(GiftPanelUiState())
    val uiState: StateFlow<GiftPanelUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<GiftPanelEvent>()
    val events: SharedFlow<GiftPanelEvent> = _events.asSharedFlow()

    /** 上次加载使用的 locale，供 retryLoad 复用 */
    private var lastLocale: String = "en"

    // ─── Init ─────────────────────────────────────────────────────────────────

    init {
        loadGifts()
        subscribeToWsEvents()
    }

    // ─── Public Actions ───────────────────────────────────────────────────────

    /**
     * 拉取礼物列表。
     *
     * - 成功 → 更新 [GiftPanelUiState.gifts]，清除 error，loading=false
     * - 失败 → 设置 [GiftPanelUiState.error]，loading=false
     *
     * @param locale IETF 语言标签（如 "en"、"ar"）
     */
    fun loadGifts(locale: String = "en") {
        lastLocale = locale
        viewModelScope.launch {
            _uiState.update { it.copy(loading = true, error = null) }
            giftRepository.listGifts(locale)
                .onSuccess { gifts ->
                    _uiState.update { it.copy(gifts = gifts, loading = false, error = null) }
                }
                .onFailure { e ->
                    if (e is CancellationException) throw e
                    _uiState.update { it.copy(loading = false, error = e.message ?: "未知错误") }
                }
        }
    }

    /**
     * 网络失败后点击重试，复用上次 locale。
     */
    fun retryLoad() = loadGifts(lastLocale)

    /**
     * 选中某礼物，更新 [GiftPanelUiState.selectedGiftId]。
     *
     * 若 [giftId] 不在当前礼物列表中，不做任何更改。
     */
    fun selectGift(giftId: String) {
        _uiState.update { state ->
            if (state.gifts.any { it.id == giftId }) {
                state.copy(selectedGiftId = giftId)
            } else {
                state
            }
        }
    }

    /**
     * 选中数量档位（1 / 10 / 66 / 520 / 786 / 1314）。
     *
     * 不限制输入值，以支持测试任意档位。
     */
    fun selectCount(count: Int) {
        _uiState.update { it.copy(selectedCount = count) }
    }

    /**
     * 选中接收者，更新 [GiftPanelUiState.selectedRecipientId]。
     *
     * 若 [userId] 不在 [GiftPanelUiState.recipients] 中，不做更改。
     */
    fun selectRecipient(userId: String) {
        _uiState.update { state ->
            if (state.recipients.any { it.userId == userId }) {
                state.copy(selectedRecipientId = userId)
            } else {
                state
            }
        }
    }

    /**
     * 更新当前在麦用户列表（由 RoomViewModel/RoomScreen 传入）。
     *
     * 若尚未选中接收者，自动选中第一个用户（默认主麦）。
     */
    fun updateRecipients(recipients: List<MicUserVO>) {
        _uiState.update { state ->
            val newSelectedId = when {
                state.selectedRecipientId != null &&
                    recipients.any { it.userId == state.selectedRecipientId } ->
                    state.selectedRecipientId
                recipients.isNotEmpty() -> recipients.first().userId
                else -> null
            }
            state.copy(recipients = recipients, selectedRecipientId = newSelectedId)
        }
    }

    /**
     * 关闭礼物面板时调用，清除选中礼物 ID（保留 balance/recipients）。
     */
    fun dismiss() {
        _uiState.update { it.copy(selectedGiftId = null) }
    }

    /**
     * 切换 Hot/All/Backpack Tab。
     */
    fun selectTab(tab: GiftTab) {
        _uiState.update { it.copy(activeTab = tab) }
    }

    /**
     * 充值按钮点击（T-30032 前占位）。
     */
    fun onRechargeClick() {
        viewModelScope.launch {
            _events.emit(GiftPanelEvent.ShowRechargeHint)
        }
    }

    // ─── WebSocket Subscription ───────────────────────────────────────────────

    /**
     * 订阅 WS 消息流，处理 `BalanceUpdated` 事件（§6.4.1 协议）。
     *
     * 解析成功 → 更新 [GiftPanelUiState.balance]
     * 解析失败 → 静默忽略
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
            when (json.get("type")?.asString) {
                "BalanceUpdated" -> {
                    val payload = json.getAsJsonObject("payload") ?: return
                    val newBalance = payload.get("diamond_balance")?.asLong ?: return
                    _uiState.update { it.copy(balance = newBalance) }
                }
            }
        } catch (e: Exception) {
            // 忽略格式错误的 WS 消息
        }
    }

    // ─── Factory ──────────────────────────────────────────────────────────────

    companion object {
        fun factory(
            giftRepository: IGiftRepository,
            wsClient: IWebSocketClient,
        ) = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T =
                GiftPanelViewModel(giftRepository, wsClient) as T
        }
    }
}
