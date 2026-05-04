package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 验收测试 — 麦克风权限懒加载（BUG-MIC-PERMISSION-TOAST / BUG-GIFT-MIC-PERMISSION，Round 6）
 *
 * 核心契约：
 * - 进房 ([RoomViewModel.joinRoom]) 期间**不应**主动调用
 *   [IMicPermissionChecker.requestMicPermission]；权限请求只能由用户点击麦位时触发。
 * - 仅当被管理员强制上麦 (ForceTakeMic) 且当前未授权时，ViewModel 才允许触发权限请求。
 *
 * 这样可以确保：
 * - 低权限受众进房不会被立即弹出系统麦克风权限弹窗 (BUG-GIFT-MIC-PERMISSION)
 * - 拒绝权限的用户仍能正常浏览房间 / 收礼物 / 看公屏
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MicPermissionLazyLoadTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var fakeClock: FakeClock
    private lateinit var fakeMicChecker: FakeMicPermissionChecker
    private lateinit var viewModel: RoomViewModel

    private val defaultSnapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 1,
        micSlots = listOf(
            MicSlotData(index = 0, userId = null, nickname = null),
            MicSlotData(index = 1, userId = null, nickname = null),
        ),
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(defaultSnapshot)
        fakeMediaService = FakeMediaService()
        fakeClock = FakeClock(currentTimeMs = 1_000_000L)
        // 关键：hasPermission = false 模拟首次进房未授权场景
        fakeMicChecker = FakeMicPermissionChecker(hasPermission = false)
        viewModel = RoomViewModel(
            wsClient = fakeWsClient,
            roomSnapshotRepository = fakeRepo,
            mediaService = fakeMediaService,
            clock = fakeClock,
            micPermissionChecker = fakeMicChecker,
        )
    }

    @Test
    fun `joinRoom does NOT trigger mic permission request when user has no permission`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Act: 进房（未授权）
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // Assert: requestMicPermission 必须 0 次调用
            assertEquals(
                "Lazy-load contract: joinRoom must not auto-request mic permission",
                0,
                fakeMicChecker.requestCallCount,
            )
            assertFalse(
                "Lazy-load contract: no pending callback after joinRoom",
                fakeMicChecker.hasPendingCallback,
            )
        }

    @Test
    fun `WS message stream during normal browsing does NOT trigger mic permission request`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeWsClient.simulateConnect()
            viewModel.joinRoom("room-1", userId = "self")
            advanceUntilIdle()

            // 模拟收到一组常见 WS 事件（聊天 / 用户加入 / 礼物等）—— 都不应触发权限请求
            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"m1","user_id":"u2","content":"hi"}}"""
            )
            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","payload":{"user_id":"u2","nickname":"Bob"}}"""
            )
            advanceUntilIdle()

            assertEquals(
                "Browsing room without clicking mic must not request permission",
                0,
                fakeMicChecker.requestCallCount,
            )
        }
}
