package com.voice.room.android.feature.room

import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.domain.room.PasswordLockedException
import com.voice.room.android.domain.room.PasswordWrongException
import com.voice.room.android.domain.room.RoomNotFoundException
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — HallViewModel 密码房弹窗 (T-30038)
 *
 * P38-01: 调用 verifyPassword 后 state 先变 Verifying（阻塞 repo）
 * P38-02: 收到 40103 (PasswordWrongException) → state 变 Error(remainingAttempts=4)
 * P38-03: 收到 42910 (PasswordLockedException) → state 变 Locked
 * P38-04: Locked.remainingMinutes 值正确（10 分钟）
 * P38-05: 成功后 hallEvents 发出 NavigateToRoom(roomId, accessToken)
 * P38-06: dismissPasswordDialog → state 变 null
 */
@OptIn(ExperimentalCoroutinesApi::class)
class HallPasswordDialogTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─────────────────────────────────────────────
    // P38-01: verifyPassword 调用后 state 先变 Verifying
    // ─────────────────────────────────────────────

    @Test
    fun `P38-01 verifyPassword sets state to Verifying before API returns`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 使用永不完成的 verifyPassword 模拟"进行中"
            val blockingRepo = object : FakeRoomRepository() {
                override suspend fun verifyPassword(
                    roomId: String,
                    password: String
                ): Result<String> = kotlinx.coroutines.awaitCancellation()
            }
            val viewModel = HallViewModel(blockingRepo)
            advanceUntilIdle() // 等待 init loadRooms 完成

            // 打开弹窗
            viewModel.openPasswordDialog("room-123")
            assertEquals(
                "openPasswordDialog should set state to Idle",
                PasswordDialogState.Idle,
                viewModel.passwordDialogState.value
            )

            // 触发 6 位输完提交
            viewModel.verifyPassword("123456")
            // 只推进一步：协程启动并执行到第一个挂起点（awaitCancellation）前先设置了 Verifying
            mainDispatcherRule.testDispatcher.scheduler.runCurrent()

            assertEquals(
                "State should be Verifying after verifyPassword is called",
                PasswordDialogState.Verifying,
                viewModel.passwordDialogState.value
            )
        }

    // ─────────────────────────────────────────────
    // P38-02: 40103 → Error(remainingAttempts=4)
    // ─────────────────────────────────────────────

    @Test
    fun `P38-02 PasswordWrongException sets state to Error with remainingAttempts 4`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult = Result.failure(PasswordWrongException(remainingAttempts = 4))
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-123")
            viewModel.verifyPassword("wrongpwd")
            advanceUntilIdle()

            val state = viewModel.passwordDialogState.value
            assertTrue(
                "State should be Error after wrong password",
                state is PasswordDialogState.Error
            )
            assertEquals(
                "remainingAttempts should be 4",
                4,
                (state as PasswordDialogState.Error).remainingAttempts
            )
        }

    // ─────────────────────────────────────────────
    // P38-03: 42910 → Locked 状态
    // ─────────────────────────────────────────────

    @Test
    fun `P38-03 PasswordLockedException sets state to Locked`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult =
                    Result.failure(PasswordLockedException(remainingMinutes = 5))
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-456")
            viewModel.verifyPassword("wrongpwd")
            advanceUntilIdle()

            assertTrue(
                "State should be Locked after too many wrong attempts",
                viewModel.passwordDialogState.value is PasswordDialogState.Locked
            )
        }

    // ─────────────────────────────────────────────
    // P38-04: Locked.remainingMinutes 值正确
    // ─────────────────────────────────────────────

    @Test
    fun `P38-04 Locked state contains correct remainingMinutes`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult =
                    Result.failure(PasswordLockedException(remainingMinutes = 10))
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-789")
            viewModel.verifyPassword("wrongpwd")
            advanceUntilIdle()

            val state = viewModel.passwordDialogState.value
            assertTrue(state is PasswordDialogState.Locked)
            assertEquals(
                "remainingMinutes should be 10",
                10,
                (state as PasswordDialogState.Locked).remainingMinutes
            )
        }

    // ─────────────────────────────────────────────
    // P38-05: 成功后 hallEvents 发出 NavigateToRoom(roomId, accessToken)
    // ─────────────────────────────────────────────

    @Test
    fun `P38-05 success emits NavigateToRoom event with correct roomId and accessToken`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult = Result.success("token-abc123")
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-999")

            // 在触发前先注册事件收集
            var receivedEvent: HallEvent? = null
            val collectJob = launch(mainDispatcherRule.testDispatcher) {
                viewModel.hallEvents.collect { event ->
                    receivedEvent = event
                }
            }

            viewModel.verifyPassword("correctpwd")
            advanceUntilIdle()
            collectJob.cancel()

            assertNotNull("Should have received a navigation event", receivedEvent)
            assertTrue(
                "Event should be NavigateToRoom",
                receivedEvent is HallEvent.NavigateToRoom
            )
            val navEvent = receivedEvent as HallEvent.NavigateToRoom
            assertEquals("roomId should match", "room-999", navEvent.roomId)
            assertEquals("accessToken should match", "token-abc123", navEvent.accessToken)
        }

    // ─────────────────────────────────────────────
    // P38-06: dismissPasswordDialog → state 变 null
    // ─────────────────────────────────────────────

    @Test
    fun `P38-06 dismissPasswordDialog sets dialogState to null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = HallViewModel(FakeRoomRepository())
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-123")
            assertNotNull(
                "Dialog state should be non-null after open",
                viewModel.passwordDialogState.value
            )

            viewModel.dismissPasswordDialog()

            assertNull(
                "Dialog state should be null after dismiss",
                viewModel.passwordDialogState.value
            )
        }

    // ─────────────────────────────────────────────
    // 边界：RoomNotFoundException → Toast + dialog 关闭
    // ─────────────────────────────────────────────

    @Test
    fun `P38-07 RoomNotFoundException shows toast and closes dialog`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult = Result.failure(RoomNotFoundException())
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("deleted-room")

            var toastEvent: HallEvent.ShowToast? = null
            val collectJob = launch(mainDispatcherRule.testDispatcher) {
                viewModel.hallEvents.collect { event ->
                    if (event is HallEvent.ShowToast) toastEvent = event
                }
            }

            viewModel.verifyPassword("somepassword")
            advanceUntilIdle()
            collectJob.cancel()

            assertNull(
                "Dialog state should be null after room not found",
                viewModel.passwordDialogState.value
            )
            assertNotNull("Should have received a toast event", toastEvent)
            assertTrue(
                "Toast message should mention room not found",
                toastEvent!!.message.contains("房间不存在")
            )
        }

    // ─────────────────────────────────────────────
    // 边界：未知错误 → Toast "网络错误，请重试"
    // ─────────────────────────────────────────────

    @Test
    fun `P38-08 unknown error shows network error toast`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply {
                verifyPasswordResult = Result.failure(RuntimeException("unknown"))
            }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            viewModel.openPasswordDialog("room-001")

            var toastEvent: HallEvent.ShowToast? = null
            val collectJob = launch(mainDispatcherRule.testDispatcher) {
                viewModel.hallEvents.collect { event ->
                    if (event is HallEvent.ShowToast) toastEvent = event
                }
            }

            viewModel.verifyPassword("somepassword")
            advanceUntilIdle()
            collectJob.cancel()

            assertNotNull("Should have received a toast", toastEvent)
            assertEquals("网络错误，请重试", toastEvent!!.message)
        }
}
