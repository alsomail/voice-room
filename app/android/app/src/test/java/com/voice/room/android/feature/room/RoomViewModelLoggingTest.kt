package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * T-30051 — WS 接收链路可观测性增强。
 *
 * 仅做日志注入，不修改业务逻辑。本套测试用于守护：
 * 1. 注入 Log.* 调用后，正常 RoomMessage 解析路径不退化（与 RoomViewModelChatTest 等价回归）。
 * 2. 非法 JSON 进入 `ws: parse failed` 分支不崩溃。
 * 3. 缺 `type` 字段进入 `ws: parse ok but type missing` 分支不崩溃。
 *
 * 由于 JVM 单测启用 `isReturnDefaultValues=true`，`android.util.Log` 调用在测试时
 * 会返回默认值而不是抛 RuntimeException，无需 ShadowLog。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomViewModelLoggingTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var viewModel: RoomViewModel

    private val snapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 1,
        micSlots = listOf(
            MicSlotData(index = 0, userId = "user-9", nickname = "Bob"),
        ),
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(snapshot)
        fakeMediaService = FakeMediaService()
        viewModel = RoomViewModel(fakeWsClient, fakeRepo, fakeMediaService)
    }

    @Test
    fun `T-30051 happy path - RoomMessage still appended after log injection`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"RoomMessage","payload":{"msg_id":"obs-1","user_id":"user-9","content":"hi"},"timestamp":1}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as RoomViewState.Success
            assertEquals("happy path 仍能渲染", 1, state.uiState.messages.size)
            assertNotNull(state.uiState.messages[0].messageId)
        }

    @Test
    fun `T-30051 invalid JSON triggers parse-failed branch and does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            val before = (viewModel.uiState.value as RoomViewState.Success).uiState.messages.size

            // 非法 JSON：进入 catch 分支并写 `ws: parse failed`。
            fakeWsClient.simulateMessage("this-is-not-json")
            advanceUntilIdle()

            val after = (viewModel.uiState.value as RoomViewState.Success).uiState.messages.size
            assertEquals("非法 JSON 不应改变 messages", before, after)
        }

    @Test
    fun `T-30051 missing type field triggers warn branch and does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()
            val before = (viewModel.uiState.value as RoomViewState.Success).uiState.messages.size

            // 合法 JSON 但缺 type：进入 `ws: parse ok but type missing` 分支。
            fakeWsClient.simulateMessage("""{"foo":"bar"}""")
            advanceUntilIdle()

            val after = (viewModel.uiState.value as RoomViewState.Success).uiState.messages.size
            assertEquals("缺 type 字段不应改变 messages", before, after)
        }
}
