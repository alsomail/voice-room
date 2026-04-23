package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.feature.room.governance.SelfGovernanceState
import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 验收测试 — 禁麦/禁言 UI 反馈 + 抱麦集成（T-30044）
 *
 * G44-01：禁麦用户点"+" 不发 WS 请求，仅 Toast
 * G44-02：禁言用户 ChatInput enabled=false（selfGovernanceState.isChatMuted）
 * G44-03：禁言用户发送按钮置灰（sendMessage 被 ViewModel 层拦截）
 * G44-04：禁言到期后 ChatInput 自动恢复（isChatMuted 随时间变化）
 * G44-05：ForceTakeMic + 无权限：自动请求；拒绝则自动 MicLeave
 * G44-06：ForceTakeMic + 已授权：直接开推流
 * G44-07：ForceLeaveMic：停止推流 + Toast + UI 状态同步
 * G44-08：被 ForceLeaveMic 后自身仍被标记为 on_mic_self=false
 */
@OptIn(ExperimentalCoroutinesApi::class)
class GovernanceUiTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var fakeClock: FakeClock
    private lateinit var fakeMicChecker: FakeMicPermissionChecker
    private lateinit var viewModel: RoomViewModel

    /** 默认快照：房间 "room-1"，麦位 slot-0 被 "other-user" 占用，slot-1 空 */
    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 5,
        micSlots = listOf(
            MicSlotData(index = 0, userId = "other-user", nickname = "OtherNick"),
            MicSlotData(index = 1, userId = null, nickname = null),
        ),
    )

    /** 固定 "now" = 1_000_000ms */
    private val fixedNow = 1_000_000L

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        fakeMediaService = FakeMediaService()
        fakeClock = FakeClock(currentTimeMs = fixedNow)
        fakeMicChecker = FakeMicPermissionChecker(hasPermission = true)
        viewModel = RoomViewModel(
            wsClient = fakeWsClient,
            roomSnapshotRepository = fakeRepo,
            mediaService = fakeMediaService,
            clock = fakeClock,
            micPermissionChecker = fakeMicChecker,
        )
    }

    // ─── G44-01：禁麦用户点"+" 不发 WS 请求，仅 Toast ─────────────────────────

    @Test
    fun `G44-01 mic muted user clicks plus - no WS TakeMic sent only ShowToast emitted`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: 进房 + WS Connected
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // 收到禁麦 WS 事件（到期时间 = fixedNow + 10min）
            val micExpiresAt = fixedNow + 600_000L
            fakeWsClient.simulateMessage(
                """{"type":"UserMuted","muteType":"mic","duration_sec":600,"expires_at":$micExpiresAt}"""
            )
            advanceUntilIdle()

            // 收集 events
            val collectedEvents = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { collectedEvents.add(it) } }

            val sentCountBefore = fakeWsClient.sentMessages.size

            // Act: 禁麦状态下请求上麦（模拟点击"+"后权限已授予进入此方法）
            viewModel.onMicPermissionGranted(slotIndex = 1)
            advanceUntilIdle()

            // Assert 1: 未发出任何新的 WS 消息
            assertEquals(
                "G44-01: No new WS message should be sent while mic is muted",
                sentCountBefore,
                fakeWsClient.sentMessages.size,
            )

            // Assert 2: 发出了 ShowToast 包含禁麦提示
            val toastEvent = collectedEvents.filterIsInstance<RoomEvent.ShowToast>()
            assertTrue(
                "G44-01: ShowToast event should be emitted",
                toastEvent.isNotEmpty(),
            )
            assertTrue(
                "G44-01: Toast message should mention muted mic",
                toastEvent.any { "禁麦" in it.message },
            )

            job.cancel()
        }

    // ─── G44-02：禁言用户 selfGovernanceState.isChatMuted = true ──────────────

    @Test
    fun `G44-02 chat muted - selfGovernanceState isChatMuted returns true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            val chatExpiresAt = fixedNow + 600_000L

            // Act: 收到禁言广播
            fakeWsClient.simulateMessage(
                """{"type":"UserMuted","muteType":"chat","duration_sec":600,"expires_at":$chatExpiresAt}"""
            )
            advanceUntilIdle()

            // Assert: selfGovernanceState 反映禁言状态
            val govState: SelfGovernanceState = viewModel.selfGovernanceState.value
            assertTrue(
                "G44-02: isChatMuted should be true when chat is muted",
                govState.isChatMuted(fakeClock.currentTimeMs),
            )
            assertFalse(
                "G44-02: isMicMuted should be false (only chat was muted)",
                govState.isMicMuted(fakeClock.currentTimeMs),
            )
        }

    // ─── G44-03：禁言状态下 sendMessage 被 ViewModel 拦截，不发 WS ─────────────

    @Test
    fun `G44-03 chat muted - sendMessage is blocked no WS sent`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            val chatExpiresAt = fixedNow + 600_000L
            fakeWsClient.simulateMessage(
                """{"type":"UserMuted","muteType":"chat","duration_sec":600,"expires_at":$chatExpiresAt}"""
            )
            advanceUntilIdle()

            val sentCountBefore = fakeWsClient.sentMessages.size

            // Act: 禁言状态下尝试发消息
            viewModel.sendMessage("hello world")
            advanceUntilIdle()

            // Assert: 未发出 SendMessage WS
            assertEquals(
                "G44-03: No SendMessage WS should be sent while chat is muted",
                sentCountBefore,
                fakeWsClient.sentMessages.size,
            )
        }

    // ─── G44-04：禁言到期后 isChatMuted 自动返回 false ────────────────────────

    @Test
    fun `G44-04 chat mute expires - isChatMuted returns false after expiry time`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: 设置禁言到期时间 = now + 1000ms
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            val chatExpiresAt = fixedNow + 1_000L  // 到期时间 = 1_001_000ms
            fakeWsClient.simulateMessage(
                """{"type":"UserMuted","muteType":"chat","duration_sec":1,"expires_at":$chatExpiresAt}"""
            )
            advanceUntilIdle()

            val govState = viewModel.selfGovernanceState.value

            // Assert at current time (fixedNow = 1_000_000): still muted
            assertTrue(
                "G44-04: isChatMuted should be true at fixedNow",
                govState.isChatMuted(fixedNow),
            )

            // Assert at expiry time (fixedNow + 1_000): exactly at expiry (not muted: nowMs >= until)
            assertFalse(
                "G44-04: isChatMuted should be false when nowMs equals expiresAt",
                govState.isChatMuted(chatExpiresAt),
            )

            // Assert after expiry (fixedNow + 2000): not muted
            assertFalse(
                "G44-04: isChatMuted should be false after expiry",
                govState.isChatMuted(fixedNow + 2_000L),
            )
        }

    // ─── G44-05：ForceTakeMic + 无权限 → 请求权限 → 拒绝 → 自动 MicLeave ──────

    @Test
    fun `G44-05 ForceTakeMic no permission denied - requestPermission called then sends MicLeave`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: 用户无权限
            fakeMicChecker.hasPermission = false
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // Act: 服务端广播被强制抱上麦（ForceTakeMic → MicTaken + forcedBy）
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":1,"userId":"self","nickname":"SelfNick","forcedBy":"admin-1"}"""
            )
            advanceUntilIdle()

            // Assert 1: requestMicPermission 被调用
            assertEquals(
                "G44-05: requestMicPermission should be called exactly once",
                1,
                fakeMicChecker.requestCallCount,
            )

            // 恢复 Connected 状态以便 send() 正常工作
            // （simulateMessage 会将 state 设置为 Message，导致 send() 静默失败）
            fakeWsClient.simulateConnect()
            val sentCountBeforeDeny = fakeWsClient.sentMessages.size

            // Act 2: 用户拒绝权限
            fakeMicChecker.denyPermission()
            advanceUntilIdle()

            // Assert 2: 自动发出 LeaveMic（包含正确 slotIndex=1）
            val leaveMicMessages = fakeWsClient.sentMessages
                .drop(sentCountBeforeDeny)
                .filter { "LeaveMic" in it }
            assertTrue(
                "G44-05: A LeaveMic WS message should be sent after permission denied",
                leaveMicMessages.isNotEmpty(),
            )
            assertTrue(
                "G44-05: LeaveMic message should contain slotIndex 1",
                leaveMicMessages.any { "\"slotIndex\":1" in it },
            )

            // Assert 3: startPublishAudio 未被调用
            assertEquals(
                "G44-05: startPublishAudio should NOT be called when permission denied",
                0,
                fakeMediaService.startPublishAudioCalls.size,
            )
        }

    // ─── G44-06：ForceTakeMic + 已授权 → 直接开推流 ───────────────────────────

    @Test
    fun `G44-06 ForceTakeMic has permission - starts publishing directly without requesting`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: 用户已有权限
            fakeMicChecker.hasPermission = true
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // Act: 服务端广播被强制抱上麦
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":1,"userId":"self","nickname":"SelfNick","forcedBy":"admin-1"}"""
            )
            advanceUntilIdle()

            // Assert 1: requestMicPermission 未被调用
            assertEquals(
                "G44-06: requestMicPermission should NOT be called when permission already granted",
                0,
                fakeMicChecker.requestCallCount,
            )

            // Assert 2: joinChannel 被调用
            assertTrue(
                "G44-06: joinChannel should be called with correct roomId and userId",
                fakeMediaService.joinChannelCalls.any { it.first == "room-1" && it.second == "self" },
            )

            // Assert 3: startPublishAudio 被调用
            assertEquals(
                "G44-06: startPublishAudio should be called exactly once",
                1,
                fakeMediaService.startPublishAudioCalls.size,
            )
        }

    // ─── G44-07：ForceLeaveMic → 停止推流 + Toast + UI 状态同步 ──────────────

    @Test
    fun `G44-07 ForceLeaveMic - stops publishing shows Toast and updates UI state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: self 已在麦上
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":1,"userId":"self","nickname":"SelfNick"}"""
            )
            advanceUntilIdle()

            val stateOnMic = viewModel.uiState.value as RoomViewState.Success
            assertTrue(
                "G44-07 precondition: self should be on mic",
                stateOnMic.uiState.isCurrentUserOnMic,
            )

            // 收集 events
            val collectedEvents = mutableListOf<RoomEvent>()
            val job = launch { viewModel.events.collect { collectedEvents.add(it) } }

            // Act: 服务端广播被强制踢下麦
            fakeWsClient.simulateMessage(
                """{"type":"MicLeft","slotIndex":1,"userId":"self","forcedBy":"admin-1"}"""
            )
            advanceUntilIdle()

            // Assert 1: stopPublishAudio 被调用
            assertTrue(
                "G44-07: stopPublishAudio should be called on ForceLeaveMic",
                fakeMediaService.stopPublishAudioCalls.isNotEmpty(),
            )

            // Assert 2: leaveChannel 被调用
            assertTrue(
                "G44-07: leaveChannel should be called on ForceLeaveMic",
                fakeMediaService.leaveChannelCalls.isNotEmpty(),
            )

            // Assert 3: ShowToast "你已被抱下麦"
            val forcedLeaveToasts = collectedEvents
                .filterIsInstance<RoomEvent.ShowToast>()
                .filter { "抱下麦" in it.message }
            assertTrue(
                "G44-07: ShowToast with '抱下麦' should be emitted on ForceLeaveMic",
                forcedLeaveToasts.isNotEmpty(),
            )

            // Assert 4: isCurrentUserOnMic = false
            val stateAfter = viewModel.uiState.value as RoomViewState.Success
            assertFalse(
                "G44-07: isCurrentUserOnMic should be false after ForceLeaveMic",
                stateAfter.uiState.isCurrentUserOnMic,
            )

            job.cancel()
        }

    // ─── G44-08：被 ForceLeaveMic 后 on_mic_self 标记为 false ─────────────────

    @Test
    fun `G44-08 ForceLeaveMic - onMicSelf is marked false after forced leave`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: self 已在麦上
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // 将 self 上麦
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":1,"userId":"self","nickname":"SelfNick"}"""
            )
            advanceUntilIdle()

            // 确认上麦成功
            val onMicState = viewModel.uiState.value as RoomViewState.Success
            assertTrue(
                "G44-08 precondition: isCurrentUserOnMic should be true after MicTaken",
                onMicState.uiState.isCurrentUserOnMic,
            )

            // Act: 服务端广播 ForceLeaveMic
            fakeWsClient.simulateMessage(
                """{"type":"MicLeft","slotIndex":1,"userId":"self","forcedBy":"admin-1"}"""
            )
            advanceUntilIdle()

            // Assert: isCurrentUserOnMic = false
            val stateAfter = viewModel.uiState.value as RoomViewState.Success
            assertFalse(
                "G44-08: isCurrentUserOnMic should be false after ForceLeaveMic",
                stateAfter.uiState.isCurrentUserOnMic,
            )

            // Assert: 麦位 slot-1 已清空
            val slot1 = stateAfter.uiState.micSlots.find { it.index == 1 }
            assertFalse(
                "G44-08: mic slot 1 should not be occupied after ForceLeaveMic",
                slot1?.isOccupied ?: false,
            )
        }
}
