package com.voice.room.android.feature.gift

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.google.gson.JsonParser
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.core.ws.event.SendGiftResultEvent
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.gift.MicUserVO
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/**
 * 礼物面板 ViewModel (T-30028 / T-30030)
 *
 * 职责：
 * - [loadGifts] 拉取礼物列表（打开面板时调用，内存缓存 60s 由 Repository 处理）
 * - 订阅 [IWebSocketClient.state] 处理 `BalanceUpdated` 实时余额更新
 * - 订阅 `SendGiftResult` WS 消息，驱动 [sendGift] 的幂等结果处理
 * - [selectGift] / [selectCount] / [selectRecipient] 维护选中态
 * - [updateRecipients] 接收来自 RoomViewModel 的当前在麦用户列表，自动选中第一个
 * - [dismiss] 关闭面板时清除选中态
 * - [retryLoad] 网络失败后点击重试
 * - [selectTab] 切换 Hot/All Tab
 * - [onRechargeClick] 充值按钮点击（发射 [GiftPanelEvent.ShowRechargeHint]）
 * - [sendGift] 生成 UUID msg_id → WS 发送 SendGift → 等待结果(5s 超时) → 处理错误码
 *
 * ### 结构化并发
 * [CancellationException] 必须 re-throw，不得吞噬。
 *
 * @param giftRepository 礼物仓库（生产: RetrofitGiftRepository，测试: Fake）
 * @param wsClient       WebSocket 客户端（生产: OkHttpWebSocketClient，测试: FakeWebSocketClient）
 * @param roomId         当前房间 ID，[sendGift] WS 信令必须携带（默认 ""，由外部调用 [setRoomId] 注入）
 */
class GiftPanelViewModel(
    private val giftRepository: IGiftRepository,
    private val wsClient: IWebSocketClient,
    roomId: String = "",
) : ViewModel() {

    // ─── State & Events ───────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow(GiftPanelUiState())
    val uiState: StateFlow<GiftPanelUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<GiftPanelEvent>()
    val events: SharedFlow<GiftPanelEvent> = _events.asSharedFlow()

    /** 上次加载使用的 locale，供 retryLoad 复用 */
    private var lastLocale: String = "en"

    // ─── SendGift 专属字段 (T-30030) ─────────────────────────────────────────

    /** 当前房间 ID（由 setRoomId 或构造参数注入） */
    private var roomId: String = roomId

    /**
     * 连击聚合器：3s 窗口内相同礼物+接收者的多次点击共用一个 msg_id，累加 count。
     */
    private val comboAggregator = ComboAggregator()

    /**
     * SendGiftResult 内部分发流（extraBufferCapacity=16 以允许 tryEmit 非挂起）。
     *
     * [handleWsMessage] 收到 SendGiftResult 后通过 tryEmit 写入；
     * [sendGift] 中通过 [first] 读取，由 [withTimeoutOrNull] 保护 5s 超时。
     */
    private val _sendGiftResultFlow = MutableSharedFlow<SendGiftResultEvent>(extraBufferCapacity = 16)

    // ─── Init ─────────────────────────────────────────────────────────────────

    init {
        loadGifts()
        subscribeToWsEvents()
    }

    // ─── Public Actions ───────────────────────────────────────────────────────

    /**
     * 注入当前房间 ID（由 RoomScreen 在打开礼物面板时调用）。
     *
     * @param id 房间 UUID
     */
    fun setRoomId(id: String) {
        roomId = id
    }

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
     * 更新当前在麦用户列表（由 RoomViewModel/RoomScreen 传入，仅包含 on-mic 用户）。
     *
     * - 按 [MicUserVO.micIndex] 升序排序（slot=0 主麦置首，T-30029）
     * - 若已选中接收者仍在麦上，保持当前选中
     * - 若已选中接收者已下麦，自动切换到第一个（主麦，slot=0）
     * - 若列表为空，清除选中（selectedRecipientId = null）
     */
    fun updateRecipients(recipients: List<MicUserVO>) {
        val sorted = recipients.sortedBy { it.micIndex }
        _uiState.update { state ->
            val newSelectedId = when {
                state.selectedRecipientId != null &&
                    sorted.any { it.userId == state.selectedRecipientId } ->
                    state.selectedRecipientId
                sorted.isNotEmpty() -> sorted.first().userId
                else -> null
            }
            state.copy(recipients = sorted, selectedRecipientId = newSelectedId)
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

    // ─── SendGift (T-30030) ───────────────────────────────────────────────────

    /**
     * 执行送礼流程（T-30030）。
     *
     * 流程：
     * 1. 前置检查：`canSend` 必须为 true（含余额/礼物/接收者/!sending）
     * 2. 通过 [ComboAggregator] 聚合连击，获取 msg_id 与 count
     * 3. 立即 flush combo（MVP：送出后清除窗口）
     * 4. 设置 `sending=true`，禁用发送按钮
     * 5. 构造 SendGift WS 消息并发送
     * 6. 等待 `SendGiftResult`（最多 [SEND_GIFT_TIMEOUT_MS]）
     * 7. 根据 code 处理结果（参见错误码映射表）
     * 8. finally：恢复 `sending=false`
     *
     * **幂等性**：每次新 combo 生成唯一 UUID msg_id；
     * 5s 内若同一 msg_id 重复到达 Server，Server 侧幂等去重。
     *
     * **错误码处理**：
     * | code  | 动作 |
     * |-------|------|
     * | 0     | ShowToast("赠送成功") |
     * | 40290 | ShowInsufficientDialog |
     * | 40400 | Toast + DismissPanel |
     * | 40402 | Toast + 刷新礼物列表 |
     * | 40403 | Toast（面板保留） |
     * | null  | 超时 Toast |
     * | other | 通用失败 Toast |
     */
    fun sendGift() {
        val state = _uiState.value
        if (!state.canSend) return

        val gift = state.selectedGift ?: return
        val recipientId = state.selectedRecipientId ?: return

        // 连击聚合：获取本次 combo（含 msg_id 和累计 count）
        val combo = comboAggregator.press(gift.id, recipientId, state.selectedCount)
        // MVP：立即 flush，每次点击"送出"均独立发送
        comboAggregator.flush()

        val job = SendGiftJob(
            msgId = combo.msgId,
            giftId = gift.id,
            recipientId = recipientId,
            count = combo.count,
            roomId = roomId,
        )

        viewModelScope.launch {
            _uiState.update { it.copy(sending = true) }
            try {
                // 构造并发送 WS 信令（§6.4.2）
                wsClient.send(buildSendGiftJson(job))

                // 等待服务端响应（超时 5s）
                val result = withTimeoutOrNull(SEND_GIFT_TIMEOUT_MS) {
                    _sendGiftResultFlow.first { it.msgId == job.msgId }
                }

                handleSendGiftResult(result)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.emit(GiftPanelEvent.ShowToast("赠送失败，请重试"))
            } finally {
                _uiState.update { it.copy(sending = false) }
            }
        }
    }

    /**
     * 构造 SendGift WS JSON 字符串（§6.4.2 协议格式）。
     *
     * 使用 Gson [JsonObject] API 构建，避免字符串插值造成的 JSON 注入风险
     * （giftId / recipientId / roomId 含 `"` 或 `\` 时字符串模板会破坏格式）。
     *
     * ```json
     * { "type":"SendGift", "msg_id":"uuid",
     *   "payload":{ "room_id":"uuid","gift_id":"uuid","receiver_id":"uuid","count":1 } }
     * ```
     */
    private fun buildSendGiftJson(job: SendGiftJob): String {
        val payload = com.google.gson.JsonObject().apply {
            addProperty("room_id", job.roomId)
            addProperty("gift_id", job.giftId)
            addProperty("receiver_id", job.recipientId)
            addProperty("count", job.count)
        }
        return com.google.gson.JsonObject().apply {
            addProperty("type", "SendGift")
            addProperty("msg_id", job.msgId)
            add("payload", payload)
        }.toString()
    }

    /**
     * 根据 [SendGiftResultEvent.code] 处理响应。
     *
     * @param result null = 超时；非 null = 服务端回复的结果事件
     */
    private suspend fun handleSendGiftResult(result: SendGiftResultEvent?) {
        when {
            result == null ->
                _events.emit(GiftPanelEvent.ShowToast("请求超时，请重试"))

            result.code == 0 ->
                _events.emit(GiftPanelEvent.ShowToast("赠送成功"))

            result.code == 40290 ->
                _events.emit(GiftPanelEvent.ShowInsufficientDialog)

            result.code == 40403 ->
                _events.emit(GiftPanelEvent.ShowToast("接收者已下麦或离开"))

            result.code == 40402 -> {
                _events.emit(GiftPanelEvent.ShowToast("该礼物已下架"))
                loadGifts(lastLocale)
            }

            result.code == 40400 -> {
                _events.emit(GiftPanelEvent.ShowToast("你已不在房间"))
                _events.emit(GiftPanelEvent.DismissPanel)
            }

            else ->
                _events.emit(GiftPanelEvent.ShowToast("赠送失败，请重试"))
        }
    }

    // ─── WebSocket Subscription ───────────────────────────────────────────────

    /**
     * 订阅 WS 消息流，处理：
     * - `BalanceUpdated` 事件（§6.4.1）→ 更新 [GiftPanelUiState.balance]
     * - `SendGiftResult` 事件（§6.4.2）→ 写入 [_sendGiftResultFlow]（T-30030）
     *
     * 解析失败静默忽略。
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

                "SendGiftResult" -> {
                    val msgId = json.get("msg_id")?.asString ?: return
                    val code = json.get("code")?.asInt ?: return
                    _sendGiftResultFlow.tryEmit(SendGiftResultEvent(msgId = msgId, code = code))
                }
            }
        } catch (e: Exception) {
            // 忽略格式错误的 WS 消息
        }
    }

    // ─── Factory ──────────────────────────────────────────────────────────────

    companion object {
        /** SendGift 请求超时时间（毫秒）—— S30-04 */
        const val SEND_GIFT_TIMEOUT_MS = 5_000L

        fun factory(
            giftRepository: IGiftRepository,
            wsClient: IWebSocketClient,
            roomId: String = "",
        ) = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T =
                GiftPanelViewModel(giftRepository, wsClient, roomId) as T
        }
    }
}
