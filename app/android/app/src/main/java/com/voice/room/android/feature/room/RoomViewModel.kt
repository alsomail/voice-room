package com.voice.room.android.feature.room

import androidx.annotation.VisibleForTesting
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.google.gson.JsonParser
import com.voice.room.android.core.media.IMediaService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.core.ws.event.GiftReceivedEvent
import com.voice.room.android.data.local.InMemoryKickCooldownStore
import com.voice.room.android.data.local.KickCooldownStore
import com.voice.room.android.data.local.AnnouncementSeenStore
import com.voice.room.android.data.local.InMemoryAnnouncementSeenStore
import com.voice.room.android.data.model.RoomMember
import com.voice.room.android.data.room.IRoomMemberRepository
import com.voice.room.android.data.room.IRoomSnapshotRepository
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.NoOpRoomMemberRepository
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.feature.room.effect.FullscreenAnim
import com.voice.room.android.feature.room.effect.GiftEffectController
import com.voice.room.android.feature.room.effect.GiftMessageUi
import com.voice.room.android.feature.room.governance.Clock
import com.voice.room.android.feature.room.governance.SystemClock
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
    private val memberRepository: IRoomMemberRepository = NoOpRoomMemberRepository(),
    private val kickCooldownStore: KickCooldownStore = InMemoryKickCooldownStore(),
    private val announcementSeenStore: AnnouncementSeenStore = InMemoryAnnouncementSeenStore(),
    private val clock: Clock = SystemClock(),
) : ViewModel() {

    companion object {
        /** 每页加载成员数 */
        private const val PAGE_SIZE = 20

        /** 公告弹窗间隔：24 小时（毫秒） */
        private const val ANNOUNCEMENT_INTERVAL_MS = 24 * 60 * 60 * 1000L
    }

    // ─── 对外暴露的状态 ────────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow<RoomViewState>(RoomViewState.Loading)

    /** 当前房间 UI 状态流，初始值为 [RoomViewState.Loading] */
    val uiState: StateFlow<RoomViewState> = _uiState.asStateFlow()

    /**
     * 当前公告弹窗内容（T-30043）
     *
     * null = 不显示弹窗；非 null 时 UI 展示公告弹窗（[AnnouncementPopup]）。
     */
    private val _showAnnouncementPopup = MutableStateFlow<String?>(null)
    val showAnnouncementPopup: StateFlow<String?> = _showAnnouncementPopup.asStateFlow()

    /**
     * 是否显示顶部公告图标 📄（T-30043）
     *
     * true = 当前房间有非空公告；false = 无公告不显示图标。
     */
    private val _showAnnouncementIcon = MutableStateFlow(false)
    val showAnnouncementIcon: StateFlow<Boolean> = _showAnnouncementIcon.asStateFlow()

    /** 观众席 UI 状态（T-30039）：麦上列表 + 观众列表 + 分页信息 */
    private val _audienceState = MutableStateFlow(AudienceUiState())
    val audienceState: StateFlow<AudienceUiState> = _audienceState.asStateFlow()

    /** 当前被点击的成员（T-30039），供 UserActionBottomSheet 使用（T-30040 联动） */
    private val _selectedMember = MutableStateFlow<RoomMember?>(null)
    val selectedMember: StateFlow<RoomMember?> = _selectedMember.asStateFlow()

    /** 当前待踢出的目标成员（T-30040 UA40-08），供 KickReasonDialog（T-30041）使用 */
    private val _selectedKickTarget = MutableStateFlow<RoomMember?>(null)
    val selectedKickTarget: StateFlow<RoomMember?> = _selectedKickTarget.asStateFlow()

    /** 礼物特效调度控制器（T-30031）*/
    private val giftEffectController = GiftEffectController(viewModelScope)

    /** L1 弹幕消息列表（金色礼物弹幕，persistently列） */
    val giftMessages: StateFlow<List<GiftMessageUi>> = giftEffectController.giftMessages

    /** L2 麦位光圈目标用户 ID（null = 无光圈） */
    val micGlowTargetUserId: StateFlow<String?> = giftEffectController.micGlowTargetUserId

    /** L3 全屏 Lottie 特效（null = 无全屏特效） */
    val fullscreenEffect: StateFlow<FullscreenAnim?> = giftEffectController.fullscreenEffect

    /** 跳过当前全屏 L3 特效 */
    fun skipFullscreenEffect() = giftEffectController.skipFullscreen()

    private val _events = Channel<RoomEvent>(Channel.UNLIMITED)

    /** 一次性 UI 事件流（导航、Toast 等），由 Channel 保证不丢失 */
    val events: Flow<RoomEvent> = _events.receiveAsFlow()

    /** 被踢出房间状态（T-30042）；null 表示未被踢出，非 null 时 UI 展示 UserKickedDialog */
    private val _kickedState = MutableStateFlow<KickedState?>(null)
    val kickedState: StateFlow<KickedState?> = _kickedState.asStateFlow()

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
     * @param roomId      目标房间 ID
     * @param userId      当前用户 ID（用于上麦/下麦身份判断，默认空字符串）
     * @param accessToken 密码房访问令牌（[HallViewModel.verifyPassword] 返回，普通房传 null）
     */
    fun joinRoom(roomId: String, userId: String = "", accessToken: String? = null) {
        currentRoomId = roomId
        currentUserId = userId
        viewModelScope.launch {
            _uiState.value = RoomViewState.Loading
            try {
                val snapshot = roomSnapshotRepository.getRoomSnapshot(roomId)
                _uiState.value = RoomViewState.Success(snapshot.toRoomUiState())
                // T-30043: 进房后处理公告弹窗逻辑
                handleAnnouncementOnEnter(snapshot.announcement, roomId)
                val msgId = UUID.randomUUID().toString()
                val accessTokenPart =
                    if (accessToken != null) ""","access_token":"$accessToken"""" else ""
                wsClient.send(
                    """{"type":"JoinRoom","roomId":"$roomId","msgId":"$msgId"$accessTokenPart}"""
                )
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
     * 切换当前用户麦克风静音状态（T-30026）。
     *
     * - 仅在用户已上麦（isCurrentUserOnMic = true）时有效。
     * - 不发送任何 WS 消息，纯本地媒体操作。
     * - CancellationException 必须 re-throw。
     */
    fun toggleMicMute() {
        val currentState = _uiState.value as? RoomViewState.Success ?: return
        if (!currentState.uiState.isCurrentUserOnMic) return
        val willMute = !currentState.uiState.isCurrentUserMuted
        viewModelScope.launch {
            try {
                if (willMute) {
                    val result = mediaService.stopPublishAudio()
                    if (result.isFailure) throw result.exceptionOrNull()!!
                } else {
                    val result = mediaService.startPublishAudio()
                    if (result.isFailure) throw result.exceptionOrNull()!!
                }
                val updated = _uiState.value as? RoomViewState.Success ?: return@launch
                _uiState.value = RoomViewState.Success(
                    updated.uiState.copy(isCurrentUserMuted = willMute)
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("麦克风操作失败：${e.message}"))
            }
        }
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
     * 加载更多成员（分页，T-30039）。
     *
     * - 当 [AudienceUiState.hasMore] 为 false 或 [AudienceUiState.loading] 为 true 时静默忽略。
     * - 每次调用将 [AudienceUiState.currentPage] +1，然后通过 [IRoomMemberRepository.listMembers]
     *   获取该页成员并追加到 [AudienceUiState.audience]，同时更新 [AudienceUiState.hasMore]。
     * - API 错误时发出 [RoomEvent.ShowToast]，不改变分页状态。
     */
    fun loadMoreMembers() {
        val current = _audienceState.value
        if (!current.hasMore || current.loading) return

        val roomId = currentRoomId ?: return
        val nextPage = current.currentPage + 1

        viewModelScope.launch {
            _audienceState.value = current.copy(loading = true)
            try {
                val result = memberRepository.listMembers(roomId, nextPage, PAGE_SIZE)
                val existingIds = _audienceState.value.audience.map { it.id }.toSet()
                val newMembers = result.members.filter { it.id !in existingIds }
                _audienceState.value = _audienceState.value.copy(
                    audience = _audienceState.value.audience + newMembers,
                    total = result.total,
                    currentPage = nextPage,
                    hasMore = result.hasMore,
                    loading = false,
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _audienceState.value = _audienceState.value.copy(loading = false)
                _events.trySend(RoomEvent.ShowToast("加载成员失败：${e.message}"))
            }
        }
    }

    /**
     * 点击成员行回调（T-30039）。
     *
     * 更新 [selectedMember]，由 UI 层监听后打开 UserActionBottomSheet（T-30040）。
     *
     * @param member 被点击的成员
     */
    fun onMemberClick(member: RoomMember) {
        _selectedMember.value = member
    }

    // ─── 治理信令（T-30040）───────────────────────────────────────────────────

    /**
     * 任命管理员（T-30040）。
     *
     * 发出 [RoomEvent.ShowConfirmAssignAdmin] 事件，UI 层弹出确认对话框；
     * 用户确认后再调用 [confirmAssignAdmin] 发送 WS 信令。
     *
     * @param targetUserId   被任命目标的用户 ID
     * @param targetNickname 被任命目标的昵称（用于确认对话框展示）
     */
    fun assignAdmin(targetUserId: String, targetNickname: String = "") {
        _events.trySend(RoomEvent.ShowConfirmAssignAdmin(targetUserId, targetNickname))
    }

    /**
     * 确认任命管理员后发送 WS 信令（T-30040）。
     *
     * @param targetUserId 被任命目标的用户 ID
     */
    fun confirmAssignAdmin(targetUserId: String) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send(
                    """{"type":"AssignAdmin","roomId":"$roomId","targetUserId":"$targetUserId"}"""
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("任命管理员失败：${e.message}"))
            }
        }
    }

    /**
     * 卸任管理员（T-30040 R1 修复）。
     *
     * 发出 [RoomEvent.ShowConfirmRevokeAdmin] 事件，UI 层弹出确认对话框；
     * 用户确认后再调用 [confirmRevokeAdmin] 发送 WS 信令。
     * 与 [assignAdmin] 保持对称的两步确认流程。
     *
     * @param targetUserId   被卸任目标的用户 ID
     * @param targetNickname 被卸任目标的昵称（用于确认对话框展示）
     */
    fun revokeAdmin(targetUserId: String, targetNickname: String = "") {
        _events.trySend(RoomEvent.ShowConfirmRevokeAdmin(targetUserId, targetNickname))
    }

    /**
     * 确认卸任管理员后发送 WS 信令（T-30040 R1 修复）。
     *
     * @param targetUserId 被卸任目标的用户 ID
     */
    fun confirmRevokeAdmin(targetUserId: String) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send(
                    """{"type":"RevokeAdmin","roomId":"$roomId","targetUserId":"$targetUserId"}"""
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("卸任管理员失败：${e.message}"))
            }
        }
    }

    /**
     * 强制抱用户上麦（T-30040）。
     *
     * @param targetUserId 目标用户 ID
     * @param slotIndex    目标麦位下标（0-based）
     */
    fun forceTakeMic(targetUserId: String, slotIndex: Int) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send(
                    """{"type":"ForceTakeMic","roomId":"$roomId","targetUserId":"$targetUserId","slotIndex":$slotIndex}"""
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("抱上麦失败：${e.message}"))
            }
        }
    }

    /**
     * 强制将用户从麦上移除（T-30040）。
     *
     * @param targetUserId 目标用户 ID
     */
    fun forceLeaveMic(targetUserId: String) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send(
                    """{"type":"ForceLeaveMic","roomId":"$roomId","targetUserId":"$targetUserId"}"""
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("抱下麦失败：${e.message}"))
            }
        }
    }

    /**
     * 禁麦或禁言用户（T-30040 UA40-09）。
     *
     * @param targetUserId 目标用户 ID
     * @param durationSec  禁用时长（秒）：300/1800/7200/86400
     * @param muteType     禁用类型："mic"（禁麦）或 "chat"（禁言）
     */
    fun muteUser(targetUserId: String, durationSec: Int, muteType: String) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                wsClient.send(
                    """{"type":"MuteUser","roomId":"$roomId","targetUserId":"$targetUserId","duration_sec":$durationSec,"muteType":"$muteType"}"""
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("禁用操作失败：${e.message}"))
            }
        }
    }

    /**
     * 踢出用户（T-30040 UA40-08）。
     *
     * 设置 [selectedKickTarget]，触发 UI 打开 KickReasonDialog（T-30041）。
     *
     * @param member 待踢出的目标成员
     */
    fun onKickAction(member: RoomMember) {
        _selectedKickTarget.value = member
    }

    /**
     * 确认踢出用户后发送 WS 信令（T-30040 / T-30041）。
     *
     * @param targetUserId 目标用户 ID
     * @param reason       踢出原因（来自 KickReasonDialog）
     */
    fun kickUser(targetUserId: String, reason: String) {
        val roomId = currentRoomId ?: return
        viewModelScope.launch {
            try {
                val safeReason = reason
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                wsClient.send(
                    """{"type":"KickUser","roomId":"$roomId","targetUserId":"$targetUserId","reason":"$safeReason"}"""
                )
                _selectedKickTarget.value = null
                _events.trySend(RoomEvent.ShowToast("已踢出"))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _events.trySend(RoomEvent.ShowToast("踢出操作失败：${e.message}"))
            }
        }
    }

    /**
     * 仅供单元测试调用，代理 [onCleared] 以绕过 `protected` 可见性限制。
     *
     * 不在生产代码中使用。
     */
    @VisibleForTesting
    internal fun triggerOnCleared() = onCleared()

    /**
     * 用户确认"知道了"被踢出弹窗后的处理（T-30042）。
     *
     * 流程：
     * 1. 保存 cooldown 到 [kickCooldownStore]（截止时间 = now + cooldownSec * 1000ms）
     * 2. 清空 [kickedState]
     * 3. 发出 [RoomEvent.NavigateBack] 让 UI 返回大厅
     */
    fun acknowledgeKick() {
        val roomId = currentRoomId ?: return
        val kicked = _kickedState.value ?: return
        val untilMs = clock.currentTimeMillis() + kicked.cooldownSec * 1000L
        kickCooldownStore.save(roomId, untilMs)
        _kickedState.value = null
        _events.trySend(RoomEvent.NavigateBack)
    }

    /**
     * 点击顶部公告图标 📄，手动展示公告弹窗（T-30043 AN43-04）。
     *
     * 仅在当前房间有非空公告时有效。
     */
    fun onAnnouncementIconClick() {
        val currentAnnouncement = (_uiState.value as? RoomViewState.Success)
            ?.uiState?.announcement ?: return
        if (currentAnnouncement.isNotBlank()) {
            _showAnnouncementPopup.value = currentAnnouncement
        }
    }

    /**
     * 关闭公告弹窗（T-30043 AN43-08）。
     *
     * 将 [showAnnouncementPopup] 重置为 null；顶部图标 [showAnnouncementIcon] 保持不变。
     */
    fun dismissAnnouncementPopup() {
        _showAnnouncementPopup.value = null
    }

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
     * 进房后处理公告弹窗逻辑（T-30043 AN43-01/AN43-02/AN43-03）。
     *
     * - 空公告 → 不弹窗，顶部图标隐藏
     * - 非空公告 + 未看过（或超 24h）→ 弹窗并保存时间戳
     * - 非空公告 + 24h 内已看过 → 仅显示顶部图标，不弹窗
     *
     * @param announcement 当前公告文本
     * @param roomId       房间 ID
     */
    private fun handleAnnouncementOnEnter(announcement: String, roomId: String) {
        if (announcement.isBlank()) {
            _showAnnouncementIcon.value = false
            return
        }
        _showAnnouncementIcon.value = true
        val last = announcementSeenStore.get(roomId)
        val now = clock.currentTimeMillis()
        if (last == null || now - last > ANNOUNCEMENT_INTERVAL_MS) {
            _showAnnouncementPopup.value = announcement
            announcementSeenStore.save(roomId, now)
        }
    }

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
                // T-30039: 将新加入的用户追加到观众席尾部
                val userId = json.get("userId")?.asString
                val nickname = json.get("nickname")?.asString ?: ""
                val role = json.get("role")?.asString ?: "member"
                val avatarUrl = json.get("avatarUrl")?.takeIf { !it.isJsonNull }?.asString
                if (userId != null) {
                    val newMember = RoomMember(
                        id = userId,
                        nickname = nickname,
                        avatarUrl = avatarUrl,
                        role = role,
                    )
                    val aud = _audienceState.value
                    // 去重：如果已存在则不追加
                    if (aud.onMic.none { it.id == userId } && aud.audience.none { it.id == userId }) {
                        _audienceState.value = aud.copy(audience = aud.audience + newMember)
                    }
                }
            }

            "UserLeft" -> {
                _uiState.value = RoomViewState.Success(
                    state.copy(onlineCount = (state.onlineCount - 1).coerceAtLeast(0))
                )
                // T-30039: 从 onMic 或 audience 中移除该用户
                val leftUserId = json.get("userId")?.asString
                if (leftUserId != null) {
                    val aud = _audienceState.value
                    _audienceState.value = aud.copy(
                        onMic = aud.onMic.filter { it.id != leftUserId },
                        audience = aud.audience.filter { it.id != leftUserId },
                    )
                }
            }

            "MicTaken" -> {
                val slotIndex = json.get("slotIndex")?.asInt ?: return
                val userId = json.get("userId")?.asString
                val nickname = json.get("nickname")?.asString
                val newSlots = state.micSlots.map { slot ->
                    if (slot.index == slotIndex) slot.copy(userId = userId, nickname = nickname)
                    else slot
                }
                val isSelf = userId != null && userId == currentUserId && currentUserId.isNotEmpty()
                _uiState.value = RoomViewState.Success(
                    state.copy(
                        micSlots = newSlots,
                        isCurrentUserOnMic = if (isSelf) true else state.isCurrentUserOnMic,
                        isCurrentUserMuted = if (isSelf) false else state.isCurrentUserMuted,
                    )
                )
                // T-30039: 将用户从 audience 移入 onMic
                if (userId != null) {
                    val aud = _audienceState.value
                    val existing = aud.audience.find { it.id == userId }
                        ?: aud.onMic.find { it.id == userId }
                        ?: RoomMember(id = userId, nickname = nickname ?: "")
                    val updated = existing.copy(slot = slotIndex)
                    _audienceState.value = aud.copy(
                        onMic = aud.onMic.filter { it.id != userId } + updated,
                        audience = aud.audience.filter { it.id != userId },
                    )
                }

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
                val leavingUserId = json.get("userId")?.asString
                    ?: state.micSlots.getOrNull(slotIndex)?.userId

                val newSlots = state.micSlots.map { slot ->
                    if (slot.index == slotIndex) slot.copy(userId = null, nickname = null)
                    else slot
                }
                val isSelfLeaving = leavingUserId != null
                    && leavingUserId == currentUserId
                    && currentUserId.isNotEmpty()
                _uiState.value = RoomViewState.Success(
                    state.copy(
                        micSlots = newSlots,
                        isCurrentUserOnMic = if (isSelfLeaving) false else state.isCurrentUserOnMic,
                        isCurrentUserMuted = if (isSelfLeaving) false else state.isCurrentUserMuted,
                    )
                )
                // T-30039: 将用户从 onMic 移回 audience
                if (leavingUserId != null) {
                    val aud = _audienceState.value
                    val leaving = aud.onMic.find { it.id == leavingUserId }
                        ?: RoomMember(id = leavingUserId, nickname = "")
                    val backToAudience = leaving.copy(slot = null)
                    _audienceState.value = aud.copy(
                        onMic = aud.onMic.filter { it.id != leavingUserId },
                        audience = aud.audience + backToAudience,
                    )
                }

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

            "AdminChanged" -> {
                // T-30039: 更新 role 字段
                val targetUserId = json.get("userId")?.asString ?: return
                val newRole = json.get("role")?.asString ?: return
                val aud = _audienceState.value
                _audienceState.value = aud.copy(
                    onMic = aud.onMic.map { m ->
                        if (m.id == targetUserId) m.copy(role = newRole) else m
                    },
                    audience = aud.audience.map { m ->
                        if (m.id == targetUserId) m.copy(role = newRole) else m
                    },
                )
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

            "Error" -> {
                val code = json.get("code")?.asInt
                when (code) {
                    40301 -> _events.trySend(RoomEvent.ShowToast("无权操作"))
                    // 其他错误码静默忽略（后续可按需扩展）
                }
            }

            "GiftReceived" -> {
                val msgId       = json.get("msgId")?.asString ?: return
                val recordId    = json.get("giftRecordId")?.asString ?: ""
                val senderObj   = json.getAsJsonObject("sender") ?: return
                val receiverObj = json.getAsJsonObject("receiver") ?: return
                val giftObj     = json.getAsJsonObject("gift") ?: return

                val evt = GiftReceivedEvent(
                    msgId           = msgId,
                    giftRecordId    = recordId,
                    senderUserId    = senderObj.get("userId")?.asString ?: return,
                    senderNickname  = senderObj.get("nickname")?.asString ?: "",
                    senderAvatar    = senderObj.get("avatar")?.takeIf { !it.isJsonNull }?.asString,
                    receiverUserId  = receiverObj.get("userId")?.asString ?: return,
                    receiverNickname = receiverObj.get("nickname")?.asString ?: "",
                    receiverAvatar  = receiverObj.get("avatar")?.takeIf { !it.isJsonNull }?.asString,
                    giftId          = giftObj.get("id")?.asString ?: return,
                    giftCode        = giftObj.get("code")?.asString ?: "",
                    giftName        = giftObj.get("name")?.asString ?: "",
                    giftIconUrl     = giftObj.get("icon_url")?.asString ?: "",
                    giftAnimationUrl = giftObj.get("animation_url")?.takeIf { !it.isJsonNull }?.asString,
                    effectLevel     = giftObj.get("effect_level")?.asInt ?: 1,
                    count           = json.get("count")?.asInt ?: 1,
                    totalPrice      = json.get("totalPrice")?.asLong ?: 0L,
                    isReplay        = json.get("isReplay")?.asBoolean ?: false,
                )
                giftEffectController.onGiftReceived(evt)
            }

            "UserKicked" -> {
                // T-30042: 收到被踢通知，设置 kickedState（WS 服务端只推送给被踢用户）
                val reason = json.get("reason")?.asString ?: ""
                val cooldownSec = json.get("cooldown_sec")?.asInt ?: 600
                _kickedState.value = KickedState(reason = reason, cooldownSec = cooldownSec)
            }

            "UserMuted" -> {
                // T-30042: 收到被禁麦/禁言通知，WS 服务端只推送给被禁用户
                val muteType = json.get("muteType")?.asString ?: return
                val durationSec = json.get("duration_sec")?.asInt ?: 0
                val expiresAt = json.get("expires_at")?.asLong
                    ?: (clock.currentTimeMillis() + durationSec * 1000L)
                // 发出 UserMuted 事件供 MuteCountdownViewModel 消费
                if (durationSec == 0) {
                    _events.trySend(RoomEvent.UserMuted(muteType = muteType, expiresAt = null))
                } else {
                    _events.trySend(RoomEvent.UserMuted(muteType = muteType, expiresAt = expiresAt))
                }
            }

            "RoomInfoUpdated" -> {
                // T-30043: 更新房间信息（title/announcement/category）
                val newTitle = json.get("title")?.takeIf { !it.isJsonNull }?.asString
                val newAnnouncement = json.get("announcement")?.takeIf { !it.isJsonNull }?.asString

                // 更新 uiState 中的 roomName 和 announcement
                val updatedState = state.copy(
                    roomName = newTitle ?: state.roomName,
                    announcement = newAnnouncement ?: state.announcement,
                )
                _uiState.value = RoomViewState.Success(updatedState)

                // 若公告有变化且非空 → 重置 seen 并重新弹窗
                if (newAnnouncement != null && newAnnouncement != state.announcement) {
                    val roomId = currentRoomId ?: return
                    if (newAnnouncement.isNotBlank()) {
                        _showAnnouncementIcon.value = true
                        announcementSeenStore.save(roomId, clock.currentTimeMillis())
                        _showAnnouncementPopup.value = newAnnouncement
                    } else {
                        _showAnnouncementIcon.value = false
                    }
                }
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
    announcement = announcement,
)

