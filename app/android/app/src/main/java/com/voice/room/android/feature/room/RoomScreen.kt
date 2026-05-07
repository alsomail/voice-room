package com.voice.room.android.feature.room

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.EmojiEvents
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import com.voice.room.android.R
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.emptyFlow
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.gift.GiftPanelBottomSheet
import com.voice.room.android.feature.gift.GiftPanelEvent
import com.voice.room.android.feature.gift.GiftPanelUiState
import com.voice.room.android.feature.room.governance.MuteCountdownViewModel
import com.voice.room.android.feature.room.governance.MuteStatusChip
import com.voice.room.android.feature.room.governance.UserKickedDialog

/**
 * 房间页顶层 Composable (T-30009 / T-30026 / T-30028 / T-30042)
 *
 * 布局（从上到下）：
 *  - [RoomTopBar]          ← topBar（房间名、在线人数、返回按钮）
 *  - [MuteStatusChip]      ← 禁麦/禁言倒计时状态 Chip（T-30042，由 MuteCountdownViewModel 驱动）
 *  - [MicSlotsGrid]        ← 9 宫格麦位区（由 [MicPermissionHandler] 守卫点击事件）
 *  - [ChatMessageList]     ← 聊天消息列表（weight(1f)，自动填充剩余高度）
 *  - [RoomBottomBar]       ← bottomBar（输入框 + 发送 + 🎤🎁❤️🚪，T-30026）
 *  - [GiftPanelBottomSheet]← 礼物面板 BottomSheet（T-30028，🎁 点击弹出）
 *  - [UserKickedDialog]    ← 被踢出房间全屏弹窗（T-30042，kickedState 非 null 时显示）
 *
 * 纯 UI，ViewModel 逻辑通过回调参数注入。
 *
 * @param uiState                房间页 UI 状态（含 [RoomUiState.isSendingMessage]）
 * @param events                 ViewModel 一次性事件流（T-30016：监听 [RoomEvent.ClearInput]）
 * @param kickedState            被踢状态（T-30042）；非 null 时显示 [UserKickedDialog]
 * @param onAcknowledgeKick      点击"知道了"确认被踢弹窗的回调（T-30042）
 * @param muteCountdownViewModel 禁麦/禁言倒计时 ViewModel（T-30042）；null 时不显示 Chip
 * @param giftUiState            礼物面板 UI 状态（T-30028，由 GiftPanelViewModel 提供）
 * @param giftEvents             礼物面板一次性事件流（T-30028）
 * @param onBack                 点击返回按钮的回调
 * @param onSendMessage          点击发送按钮的回调，参数为消息文本
 * @param onMicSlotClick         麦位点击回调（自己已占麦位 → 触发下麦，T-30055 BUG-MIC-ONCLICK 修复）
 * @param onMicPermissionGranted 麦克风权限授予后的回调，参数为麦位 index（T-30012）
 * @param onMicToggle            点击麦克风静音切换按钮的回调（T-30026）
 * @param onLeaveRoom            确认退出房间的回调（T-30026）
 * @param onSelectGift           选中礼物回调（T-30028）
 * @param onSelectCount          数量档位选择回调（T-30028）
 * @param onSelectGiftTab        切换礼物 Tab 回调（T-30028）
 * @param onSelectRecipient      选择接收者回调，参数为选中用户 userId（T-30029）
 * @param onSendGift             送出礼物回调（T-30030 接入，T-30028 暂留空）
 * @param onGiftRechargeClick    充值按钮回调（T-30028）
 * @param onGiftRetry        网络失败后点击重试回调（T-30028 R1 修复）
 * @param onGiftPanelDismiss     关闭礼物面板回调（T-30028）
 * @param onGoToWalletClick      余额不足弹窗"去充值"按钮点击回调 → 触发 vm.onGoToWallet()（T-30032）
 * @param onNavigateToWallet     收到 NavigateToWallet 事件后的实际导航回调（T-30032）
 * @param onNavigateToRanking    点击房间菜单"榜单"入口后的导航回调（T-30033 MEDIUM-02）
 * @param modifier               可选 Modifier
 */
@Composable
fun RoomScreen(
    uiState: RoomUiState,
    events: Flow<RoomEvent> = emptyFlow(),
    kickedState: KickedState? = null,
    onAcknowledgeKick: () -> Unit = {},
    muteCountdownViewModel: MuteCountdownViewModel? = null,
    giftUiState: GiftPanelUiState = GiftPanelUiState(loading = false),
    giftEvents: kotlinx.coroutines.flow.SharedFlow<GiftPanelEvent> = MutableSharedFlow(),
    onBack: () -> Unit = {},
    onSendMessage: (String) -> Unit = {},
    onMicPermissionGranted: (slotIndex: Int) -> Unit = {},
    onMicSlotClick: (slotIndex: Int) -> Unit = {},
    onMicToggle: () -> Unit = {},
    onLeaveRoom: () -> Unit = {},
    onSelectGift: (String) -> Unit = {},
    onSelectCount: (Int) -> Unit = {},
    onSelectGiftTab: (com.voice.room.android.feature.gift.GiftTab) -> Unit = {},
    onSelectRecipient: (String) -> Unit = {},
    onSendGift: () -> Unit = {},
    onGiftRechargeClick: () -> Unit = {},
    onGiftRetry: () -> Unit = {},
    onGiftPanelDismiss: () -> Unit = {},
    onGoToWalletClick: () -> Unit = {},
    onNavigateToWallet: () -> Unit = {},
    onNavigateToRanking: () -> Unit = {},
    /** 下麦确认对话框中点击"下麦"后的回调，参数为麦位 index（T-30055 TC-MIC-00009 Step2）*/
    onConfirmLeaveMic: (slotIndex: Int) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    // T-30016: 输入框本地状态，由 ClearInput 事件驱动清空
    var localInputText by remember { mutableStateOf("") }

    // T-30028: 礼物面板显示状态（本地）
    var showGiftPanel by remember { mutableStateOf(false) }

    // T-30033 MEDIUM-02: 溢出菜单展开状态（本地）
    var showOverflowMenu by remember { mutableStateOf(false) }

    // T-30055 TC-MIC-00009 Step2: 下麦确认对话框状态（本地）
    var leaveMicConfirmSlotIndex by remember { mutableStateOf<Int?>(null) }

    // T-30042: 收集禁麦/禁言到期时间戳
    val micExpiresAt by (muteCountdownViewModel?.micExpiresAt ?: kotlinx.coroutines.flow.MutableStateFlow(null)).collectAsState()
    val chatExpiresAt by (muteCountdownViewModel?.chatExpiresAt ?: kotlinx.coroutines.flow.MutableStateFlow(null)).collectAsState()

    // 监听 ViewModel 事件
    LaunchedEffect(Unit) {
        events.collect { event ->
            when (event) {
                is RoomEvent.ClearInput -> {
                    localInputText = ""
                }
                is RoomEvent.UserMuted -> {
                    // T-30042: 转发给 MuteCountdownViewModel
                    val expiresAt = event.expiresAt
                    if (expiresAt == null) {
                        if (event.muteType == "mic") muteCountdownViewModel?.clearMic()
                        else muteCountdownViewModel?.clearChat()
                    } else {
                        if (event.muteType == "mic") muteCountdownViewModel?.startMicCountdown(expiresAt)
                        else muteCountdownViewModel?.startChatCountdown(expiresAt)
                    }
                }
                is RoomEvent.ShowLeaveMicConfirmDialog -> {
                    // T-30055 TC-MIC-00009 Step2: 弹出下麦确认对话框
                    leaveMicConfirmSlotIndex = event.slotIndex
                }
                else -> { /* 其他事件由调用方通过 events flow 处理 */ }
            }
        }
    }

    Scaffold(
        modifier = modifier.imePadding(),
        topBar = {
            RoomTopBar(
                roomName = uiState.roomName,
                onlineCount = uiState.onlineCount,
                onBack = onBack,
                extraActions = {
                    // T-30033 MEDIUM-02: 溢出菜单 — "榜单"入口
                    IconButton(
                        onClick = { showOverflowMenu = true },
                        modifier = Modifier.testTag("room_overflow_menu_button"),
                    ) {
                        Icon(
                            imageVector = Icons.Filled.MoreVert,
                            contentDescription = stringResource(id = R.string.room_overflow_more),
                            tint = MenaColors.OnBackground,
                        )
                    }
                    DropdownMenu(
                        expanded = showOverflowMenu,
                        onDismissRequest = { showOverflowMenu = false },
                    ) {
                        DropdownMenuItem(
                            text = { Text(stringResource(id = R.string.room_menu_ranking)) },
                            leadingIcon = {
                                Icon(
                                    imageVector = Icons.Filled.EmojiEvents,
                                    contentDescription = stringResource(id = R.string.room_menu_ranking),
                                )
                            },
                            onClick = {
                                showOverflowMenu = false
                                onNavigateToRanking()
                            },
                            modifier = Modifier.testTag("room_menu_ranking"),
                        )
                    }
                },
            )
        },
        bottomBar = {
            RoomBottomBar(
                inputText = localInputText,
                onInputTextChange = { localInputText = it },
                isSending = uiState.isSendingMessage,
                onSendMessage = { text ->
                    onSendMessage(text)
                    // 不立即清空：等待 ViewModel 发出 ClearInput 事件（成功后）
                    // 失败时保留输入内容，允许重试（T-30016 验收标准 3）
                },
                isOnMic = uiState.isCurrentUserOnMic,
                isMicMuted = uiState.isCurrentUserMuted,
                onMicToggle = onMicToggle,
                onLeaveRoom = onLeaveRoom,
                // T-30028: 🎁 按钮点击 → 弹出 GiftPanelBottomSheet
                onGiftClick = { showGiftPanel = true },
                // 缺陷 #2 修复：表情按钮回调上移（暂留空，后续 emoji 模块接入）
                onEmojiClick = { /* TODO: emoji panel - emoji feature pending */ },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(MenaColors.Background)   // T-30025: 深色背景
                .padding(padding),
        ) {
            // T-30042: 禁麦/禁言状态 Chip（有则展示，mic 和 chat 独立）
            micExpiresAt?.let { expiresAt ->
                MuteStatusChip(
                    muteType = "mic",
                    expiresAtMs = expiresAt,
                    onExpired = { muteCountdownViewModel?.clearMic() },
                )
            }
            chatExpiresAt?.let { expiresAt ->
                MuteStatusChip(
                    muteType = "chat",
                    expiresAtMs = expiresAt,
                    onExpired = { muteCountdownViewModel?.clearChat() },
                )
            }

            MicPermissionHandler(onPermissionGranted = onMicPermissionGranted) { onPermissionRequest ->
                MicSlotsGrid(
                    slots = uiState.micSlots,
                    // T-30055 BUG-MIC-ONCLICK: 根据槽位状态路由点击事件
                    //   · 已占用（任何人）→ 交由 ViewModel.onMicSlotClick 路由（自己=下麦，他人=no-op）
                    //   · 空槽位         → 走麦克风权限检查流程（上麦）
                    onMicSlotClick = { slotIndex ->
                        val slot = uiState.micSlots.getOrNull(slotIndex)
                        if (slot?.userId != null) {
                            onMicSlotClick(slotIndex)
                        } else {
                            onPermissionRequest(slotIndex)
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .wrapContentHeight(),        // T-30025: 移除 height(240.dp) 硬编码
                )
            }
            ChatMessageList(
                messages = uiState.messages,
                modifier = Modifier.weight(1f),
            )
        }
    }

    // T-30042: 被踢出房间弹窗（全屏覆盖，不可外部关闭）
    kickedState?.let { ks ->
        UserKickedDialog(
            state = ks,
            onAcknowledge = onAcknowledgeKick,
        )
    }

    // T-30055 TC-MIC-00009 Step2: 下麦确认对话框
    // 用户点击自己麦位图标后弹出；点击"下麦"确认后调用 onConfirmLeaveMic 发出 LeaveMic 信令。
    leaveMicConfirmSlotIndex?.let { slotIdx ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { leaveMicConfirmSlotIndex = null },
            title   = { Text(text = "下麦") },
            text    = { Text(text = "确认离开麦位吗？") },
            confirmButton = {
                androidx.compose.material3.TextButton(
                    onClick = {
                        leaveMicConfirmSlotIndex = null
                        onConfirmLeaveMic(slotIdx)
                    },
                ) {
                    Text(text = "下麦")
                }
            },
            dismissButton = {
                androidx.compose.material3.TextButton(
                    onClick = { leaveMicConfirmSlotIndex = null },
                ) {
                    Text(text = "取消")
                }
            },
        )
    }

    // T-30028: 礼物面板 BottomSheet
    if (showGiftPanel) {
        GiftPanelBottomSheet(
            uiState = giftUiState,
            events = giftEvents,
            onDismiss = {
                showGiftPanel = false
                onGiftPanelDismiss()
            },
            onSelectGift = onSelectGift,
            onSelectCount = onSelectCount,
            onSelectTab = onSelectGiftTab,
            onSelectRecipient = onSelectRecipient,
            onRetry = onGiftRetry,
            onSendGift = onSendGift,
            onRechargeClick = onGiftRechargeClick,
            onGoToWalletClick = onGoToWalletClick,
            onNavigateToWallet = onNavigateToWallet,
        )
    }
}

// ─────────────────────────────────────────────
// Preview
// ─────────────────────────────────────────────

@Preview(showBackground = true, name = "RoomScreen — 预览")
@Composable
private fun RoomScreenPreview() {
    RoomScreen(
        uiState = RoomUiState(
            roomId = "preview-room",
            roomName = "欢迎来到语聊房",
            onlineCount = 12,
            micSlots = List(9) { index ->
                when (index) {
                    0 -> MicSlotUi(index = 0, userId = "u1", nickname = "Alice")
                    1 -> MicSlotUi(index = 1, userId = "u2", nickname = "Bob", isMuted = true)
                    else -> MicSlotUi(index = index)
                }
            },
            messages = listOf(
                ChatMessageUi(messageId = "m1", senderNickname = "Alice", content = "大家好！", timestamp = 0L),
                ChatMessageUi(messageId = "m2", senderNickname = "Bob", content = "欢迎~", timestamp = 1L),
            ),
        ),
    )
}
