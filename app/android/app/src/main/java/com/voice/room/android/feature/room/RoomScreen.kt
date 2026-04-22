package com.voice.room.android.feature.room

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.emptyFlow
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.gift.GiftPanelBottomSheet
import com.voice.room.android.feature.gift.GiftPanelEvent
import com.voice.room.android.feature.gift.GiftPanelUiState

/**
 * 房间页顶层 Composable (T-30009 / T-30026 / T-30028)
 *
 * 布局（从上到下）：
 *  - [RoomTopBar]          ← topBar（房间名、在线人数、返回按钮）
 *  - [MicSlotsGrid]        ← 9 宫格麦位区（由 [MicPermissionHandler] 守卫点击事件）
 *  - [ChatMessageList]     ← 聊天消息列表（weight(1f)，自动填充剩余高度）
 *  - [RoomBottomBar]       ← bottomBar（输入框 + 发送 + 🎤🎁❤️🚪，T-30026）
 *  - [GiftPanelBottomSheet]← 礼物面板 BottomSheet（T-30028，🎁 点击弹出）
 *
 * 纯 UI，ViewModel 逻辑通过回调参数注入。
 *
 * @param uiState                房间页 UI 状态（含 [RoomUiState.isSendingMessage]）
 * @param events                 ViewModel 一次性事件流（T-30016：监听 [RoomEvent.ClearInput]）
 * @param giftUiState            礼物面板 UI 状态（T-30028，由 GiftPanelViewModel 提供）
 * @param giftEvents             礼物面板一次性事件流（T-30028）
 * @param onBack                 点击返回按钮的回调
 * @param onSendMessage          点击发送按钮的回调，参数为消息文本
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
 * @param modifier               可选 Modifier
 */
@Composable
fun RoomScreen(
    uiState: RoomUiState,
    events: Flow<RoomEvent> = emptyFlow(),
    giftUiState: GiftPanelUiState = GiftPanelUiState(loading = false),
    giftEvents: kotlinx.coroutines.flow.SharedFlow<GiftPanelEvent> = MutableSharedFlow(),
    onBack: () -> Unit = {},
    onSendMessage: (String) -> Unit = {},
    onMicPermissionGranted: (slotIndex: Int) -> Unit = {},
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
    modifier: Modifier = Modifier,
) {
    // T-30016: 输入框本地状态，由 ClearInput 事件驱动清空
    var localInputText by remember { mutableStateOf("") }

    // T-30028: 礼物面板显示状态（本地）
    var showGiftPanel by remember { mutableStateOf(false) }

    // 监听 ViewModel 事件：成功发送后清空输入框
    LaunchedEffect(Unit) {
        events.collect { event ->
            if (event is RoomEvent.ClearInput) {
                localInputText = ""
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
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(MenaColors.Background)   // T-30025: 深色背景
                .padding(padding),
        ) {
            MicPermissionHandler(onPermissionGranted = onMicPermissionGranted) { onMicSlotClick ->
                MicSlotsGrid(
                    slots = uiState.micSlots,
                    onMicSlotClick = onMicSlotClick,
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
