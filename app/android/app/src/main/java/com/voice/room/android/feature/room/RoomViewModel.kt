package com.voice.room.android.feature.room

import androidx.annotation.VisibleForTesting
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.google.gson.JsonParser
import com.voice.room.android.core.media.IMediaService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.data.room.IRoomSnapshotRepository
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import java.util.UUID

/**
 * 房间页 ViewModel（T-30010 / T-30013）
 *
 * 职责：
 * - 调用 [IRoomSnapshotRepository] 获取房间初始快照（HTTP）
 * - 通过 [IWebSocketClient.state] 订阅 WS 事件，实时更新 UI 状态
 * - 暴露 [uiState] (StateFlow) 和 [events] (Channel Flow) 供 UI 层消费
 * - 管理 joinRoom / leaveRoom / sendMessage 生命周期
 * - 上麦/下麦信令发送 + RTC 媒体服务调用（T-30013）
 *
 * ### WS 消息处理对应关系
 * | 消息 type        | 处理逻辑                                                        |
 * |-----------------|----------------------------------------------------------------|
 * | UserJoined      | onlineCount++                                                  |
 * | UserLeft        | onlineCount--（最低为 0）                                        |
 * | MicTaken        | 对应 slotIndex 的 userId/nickname 更新；若为自己则调用 mediaService |
 * | MicLeft         | 对应 slotIndex 清空 (userId=null)；若为自己则调用 mediaService     |
 * | MessageReceived | 追加 chatMessages，按 msgId 去重                                 |
 * | RoomClosed      | 发出 [RoomEvent.NavigateBack]                                   |
 *
 * @param wsClient               WS 客户端（生产: OkHttpWebSocketClient，测试: FakeWebSocketClient）
 * @param roomSnapshotRepository 房间快照仓库（生产: RetrofitRoomSnapshotRepository，测试: FakeRoomSnapshotRepository）
 * @param mediaService           媒体服务（生产: NoOpMediaService，测试: FakeMediaService）
 */
class RoomViewModel(
    private val wsClient: IWebSocketClient,
    private val roomSnapshotRepository: IRoomSnapshotRepository,
    private val mediaService: IMediaService = NoOpMediaService(),
) : ViewModel() {

    // ─── 对外暴露的状态 ────────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow<RoomViewState>(RoomViewState.Loading)

    /** 当前房间 UI 状态流，初始值为 [RoomViewState.Loading] */
    val uiState: StateFlow<RoomViewState> = _uiState.asStateFlow()

    private val _events = Channel<RoomEvent>(Channel.UNLIMITED)

    /** 一次性 UI 事件流（导航、Toast 等），由 Channel 保证不丢失 */
    val events: Flow<RoomEvent> = _events.receiveAsFlow()

    // ─── 内部状态 ──────────────────────────────────────────────────────────────

    private var currentRoomId: String? = null

    /** 当前登录用户 ID，由 [joinRoom] 传入，用于区分上麦/下麦事件是否属于自己 */
    private var currentUserId: String = ""

    /** 已处理过的消息 ID 集合，用于去重 */
    // TODO(T-30010): seenMsgIds 无界增长，长时间在线时内存持续上升。
    //                MVP 可接受；后续应改为 LRU 固定上限（如 maxSize=1000）或定期清理。
    private val seenMsgIds = mutableSetOf<String>()

    // ─── 初始化：订阅 WS 消息 ──────────────────────────────────────────────────

    init {
        observeWsMessages()
    }

    // ─── 公开操作 ──────────────────────────────────────────────────────────────

    /**
     * 进入房间：获取 HTTP 快照 → 初始化 UI 状态 → 发送 JoinRoom WS 消息。
     *
     * @param roomId 目标房间 ID
     * @param userId 当前用户 ID（用于上麦/下麦身份判断，默认空字符串）
     */
    fun joinRoom(roomId: String, userId: String = "") {
        currentRoomId = roomId
        currentUserId = userId
        viewModelScope.launch {
            _uiState.value = RoomViewState.Loading
            try {
                val snapshot = roomSnapshotRepository.getRoomSnapshot(roomId)
                _uiState.value = RoomViewState.Success(snapshot.toRoomUiState())
                val msgId = UUID.randomUUID().toString()
                wsClient.send("""{"type":"JoinRoom","roomId":"$roomId","msgId":"$msgId"}""")
            } catch (e: CancellationException) {
                throw e  // 必须 rethrow，保持协程取消语义
            } catch (e: Exception) {
                _uiState.value = RoomViewState.Error(e.message ?: "Unknown error")
            }
        }
    }

    /**
     * 麦克风权限授予后的处理入口（T-30012 / T-30013）。
     *
     * 权限已授予 → 向服务端发送 TakeMic 信令。
     * 服务端收到后广播 MicTaken，ViewModel 再调用 RTC mediaService。
     *
     * @param slotIndex 麦位下标（0-based）
     */
    fun onMicPermissionGranted(slotIndex: Int) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send("""{"type":"TakeMic","roomId":"$roomId","slotIndex":$slotIndex}""")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("上麦失败：${e.message}"))
            }
        }
    }

    /**
     * 麦位点击路由（T-30013）。
     *
     * - 若点击的是**自己**的占用麦位 → 执行下麦（发送 LeaveMic 信令）
     * - 若点击空麦位或他人麦位 → 不做操作（空麦位由上层 MicPermissionHandler 触发权限流程）
     *
     * @param slotIndex 麦位下标（0-based）
     */
    fun onMicSlotClick(slotIndex: Int) {
        val currentState = _uiState.value as? RoomViewState.Success ?: return
        val slot = currentState.uiState.micSlots.getOrNull(slotIndex) ?: return

        if (slot.userId != null && slot.userId == currentUserId && currentUserId.isNotEmpty()) {
            leaveMic(slotIndex)
        }
        // 空麦位 / 他人麦位：不操作
    }

    /**
     * 离开房间：发送 LeaveRoom WS 消息 → 断开 WS 连接。
     *
     * 此方法为同步调用（无 suspend），可在 [onCleared] 中安全调用。
     */
    fun leaveRoom() {
        currentRoomId?.let { roomId ->
            wsClient.send("""{"type":"LeaveRoom","roomId":"$roomId"}""")
        }
        wsClient.disconnect()
    }

    /**
     * 发送聊天消息（需已通过 [joinRoom] 设置 currentRoomId）。
     *
     * 流程：
     * 1. 空白内容或无活跃房间 → 提前返回（不发送）
     * 2. 设置 [RoomUiState.isSendingMessage] = true，禁用输入框发送按钮
     * 3. 通过 [IWebSocketClient.send] 发送 SendMessage 信令
     * 4. 成功 → 发出 [RoomEvent.ClearInput]；失败 → 发出 [RoomEvent.ShowToast]（不清空，允许重试）
     * 5. finally 复位 [RoomUiState.isSendingMessage] = false
     *
     * @param content 消息正文（空白字符串将被忽略）
     */
    fun sendMessage(content: String) {
        if (content.isBlank()) return
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            updateSendingState(true)
            try {
                val msgId = UUID.randomUUID().toString()
                wsClient.send(
                    """{"type":"SendMessage","roomId":"$roomId","content":"$content","msgId":"$msgId"}"""
                )
                _events.trySend(RoomEvent.ClearInput)
            } catch (e: CancellationException) {
                throw e  // 必须 rethrow，保持协程取消语义
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("发送失败：${e.message}"))
                // 失败：不清空输入，允许用户重试
            } finally {
                updateSendingState(false)
            }
        }
    }

    /**
     * ViewModel 销毁时自动调用 [leaveRoom]，确保资源释放。
     *
     * 已恢复为 `protected`（Kotlin 默认），不再暴露为 `public`。
     * 测试请通过 [triggerOnCleared] 间接调用。
     */
    override fun onCleared() {
        super.onCleared()
        leaveRoom()
    }

    /**
     * 仅供单元测试调用，代理 [onCleared] 以绕过 `protected` 可见性限制。
     *
     * 不在生产代码中使用。
     */
    @VisibleForTesting
    internal fun triggerOnCleared() = onCleared()

    // ─── 私有：下麦信令发送 ────────────────────────────────────────────────────

    private fun leaveMic(slotIndex: Int) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send("""{"type":"LeaveMic","roomId":"$roomId","slotIndex":$slotIndex}""")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("下麦失败：${e.message}"))
            }
        }
    }

    // ─── 私有：发送中状态更新 ──────────────────────────────────────────────────

    /**
     * 更新 [RoomUiState.isSendingMessage] 字段。
     *
     * 仅当当前状态为 [RoomViewState.Success] 时生效；Loading / Error 状态时静默忽略。
     */
    private fun updateSendingState(isSending: Boolean) {
        val current = _uiState.value as? RoomViewState.Success ?: return
        _uiState.value = RoomViewState.Success(current.uiState.copy(isSendingMessage = isSending))
    }

    // ─── 私有：WS 消息订阅 ─────────────────────────────────────────────────────

    private fun observeWsMessages() {
        viewModelScope.launch {
            wsClient.state.collect { wsState ->
                if (wsState is WebSocketState.Message) {
                    handleWsMessage(wsState.text)
                }
            }
        }
    }

    /**
     * 解析 WS 消息 JSON，根据 type 分发到对应处理逻辑。
     *
     * 使用 Gson [JsonParser] 解析，该库已通过 retrofit converter-gson 引入。
     * 若解析失败或 type 未知则静默忽略。
     */
    private fun handleWsMessage(raw: String) {
        val json = try {
            JsonParser.parseString(raw).asJsonObject
        } catch (e: Exception) {
            return
        }

        val type = json.get("type")?.asString ?: return

        // 非 Success 状态时忽略所有 WS 消息（joinRoom 尚未完成）
        val currentState = _uiState.value as? RoomViewState.Success ?: return
        val state = currentState.uiState

        when (type) {
            "UserJoined" -> {
                _uiState.value = RoomViewState.Success(
                    state.copy(onlineCount = state.onlineCount + 1)
                )
            }

            "UserLeft" -> {
                _uiState.value = RoomViewState.Success(
                    state.copy(onlineCount = (state.onlineCount - 1).coerceAtLeast(0))
                )
            }

            "MicTaken" -> {
                val slotIndex = json.get("slotIndex")?.asInt ?: return
                val userId = json.get("userId")?.asString
                val nickname = json.get("nickname")?.asString
                val newSlots = state.micSlots.map { slot ->
                    if (slot.index == slotIndex) slot.copy(userId = userId, nickname = nickname)
                    else slot
                }
                _uiState.value = RoomViewState.Success(state.copy(micSlots = newSlots))

                // 若是当前用户上麦成功，调用 RTC mediaService
                if (userId != null && userId == currentUserId && currentUserId.isNotEmpty()) {
                    val roomId = currentRoomId ?: return
                    viewModelScope.launch {
                        try {
                            val joinResult = mediaService.joinChannel(roomId, userId)
                            if (joinResult.isFailure) {
                                _events.trySend(
                                    RoomEvent.ShowToast(
                                        "加入频道失败：${joinResult.exceptionOrNull()?.message}"
                                    )
                                )
                                return@launch
                            }
                            val publishResult = mediaService.startPublishAudio()
                            if (publishResult.isFailure) {
                                _events.trySend(
                                    RoomEvent.ShowToast(
                                        "开启推流失败：${publishResult.exceptionOrNull()?.message}"
                                    )
                                )
                            }
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            _events.trySend(RoomEvent.ShowToast("上麦媒体操作异常：${e.message}"))
                        }
                    }
                }
            }

            "MicLeft" -> {
                val slotIndex = json.get("slotIndex")?.asInt ?: return
                // 在清空前记录该槽位原有 userId，用于判断是否需要调用 mediaService
                val leavingUserId = state.micSlots.getOrNull(slotIndex)?.userId

                val newSlots = state.micSlots.map { slot ->
                    if (slot.index == slotIndex) slot.copy(userId = null, nickname = null)
                    else slot
                }
                _uiState.value = RoomViewState.Success(state.copy(micSlots = newSlots))

                // 若是当前用户下麦，停止推流并离开频道
                if (leavingUserId != null
                    && leavingUserId == currentUserId
                    && currentUserId.isNotEmpty()
                ) {
                    viewModelScope.launch {
                        try {
                            mediaService.stopPublishAudio()
                            mediaService.leaveChannel()
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            // 下麦清理失败静默处理
                        }
                    }
                }
            }

            "MessageReceived" -> {
                val msgId = json.get("msgId")?.asString ?: return
                if (seenMsgIds.contains(msgId)) return
                seenMsgIds.add(msgId)

                val senderNickname = json.get("senderNickname")?.asString ?: ""
                val content = json.get("content")?.asString ?: return  // content 缺失 → 静默忽略（RM-05）
                val timestamp = json.get("timestamp")?.asLong ?: 0L

                val newMsg = ChatMessageUi(
                    messageId = msgId,
                    senderNickname = senderNickname,
                    content = content,
                    timestamp = timestamp,
                )
                _uiState.value = RoomViewState.Success(
                    state.copy(messages = state.messages + newMsg)
                )
            }

            "RoomClosed" -> {
                _events.trySend(RoomEvent.NavigateBack)
            }
        }
    }
}

// ─── 扩展函数：RoomSnapshot → RoomUiState ─────────────────────────────────────

/**
 * 将 HTTP 响应数据模型 [RoomSnapshot] 转换为 UI 状态 [RoomUiState]。
 *
 * - 补全 9 个麦位（不足的以空 [MicSlotUi] 填充）
 * - 其余字段直接映射
 */
fun RoomSnapshot.toRoomUiState(): RoomUiState = RoomUiState(
    roomId = roomId,
    roomName = roomName,
    onlineCount = onlineCount,
    micSlots = List(9) { index ->
        val slotData: MicSlotData? = micSlots.find { it.index == index }
        MicSlotUi(
            index = index,
            userId = slotData?.userId,
            nickname = slotData?.nickname,
        )
    },
    messages = emptyList(),
)

