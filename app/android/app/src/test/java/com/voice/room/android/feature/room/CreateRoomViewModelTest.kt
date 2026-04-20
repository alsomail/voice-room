package com.voice.room.android.feature.room

import androidx.paging.PagingSource
import androidx.paging.PagingState
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — CreateRoomViewModel (T-30007)
 *
 * C01: 初始状态为 Idle
 * C02: 标题为空时 createRoom 返回校验错误
 * C03: 标题超过 30 字符时 createRoom 返回校验错误
 * C04: password 类型但密码为空时 createRoom 返回校验错误
 * C05: 普通类型房间创建成功，状态变为 Success(roomId)
 * C06: 密码类型房间创建成功，状态变为 Success(roomId)
 * C07: API 失败时，状态变为 Error(message)
 * C08: 创建中途不能重复提交（Loading 状态下忽略二次调用）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class CreateRoomViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─────────────────────────────────────────────
    // Fake Repository（测试专用）
    // ─────────────────────────────────────────────

    /**
     * 普通成功 Fake：createRoom 立即返回 Result.success(roomId)
     */
    private class FakeSuccessRepository(
        val returnedRoomId: String = "room-test-001"
    ) : IRoomRepository {
        var createRoomCallCount = 0
        var lastTitle: String? = null
        var lastType: String? = null
        var lastPassword: String? = null

        override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
            Result.success(RoomsPage(total = 0, page = 1, items = emptyList()))

        override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
            NullPagingSource()

        override suspend fun createRoom(
            title: String,
            type: String,
            password: String?
        ): Result<String> {
            createRoomCallCount++
            lastTitle = title
            lastType = type
            lastPassword = password
            return Result.success(returnedRoomId)
        }
    }

    /**
     * 失败 Fake：createRoom 返回 Result.failure
     */
    private class FakeFailureRepository(
        val errorMessage: String = "用户已有活跃房间"
    ) : IRoomRepository {
        override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
            Result.success(RoomsPage(total = 0, page = 1, items = emptyList()))

        override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
            NullPagingSource()

        override suspend fun createRoom(
            title: String,
            type: String,
            password: String?
        ): Result<String> = Result.failure(Exception(errorMessage))
    }

    /**
     * 阻塞 Fake：createRoom 永久挂起，用于测试 Loading 幂等性（C08）
     */
    private class FakeBlockingRepository : IRoomRepository {
        override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
            Result.success(RoomsPage(total = 0, page = 1, items = emptyList()))

        override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
            NullPagingSource()

        override suspend fun createRoom(
            title: String,
            type: String,
            password: String?
        ): Result<String> = awaitCancellation()
    }

    /** 空 PagingSource（测试中不需要分页功能） */
    private class NullPagingSource : PagingSource<Int, RoomItem>() {
        override fun getRefreshKey(state: PagingState<Int, RoomItem>): Int? = null
        override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> =
            LoadResult.Page(data = emptyList(), prevKey = null, nextKey = null)
    }

    // ─────────────────────────────────────────────
    // C01: 初始状态为 Idle
    // ─────────────────────────────────────────────

    @Test
    fun `C01 initial state is Idle`() {
        val viewModel = CreateRoomViewModel(FakeSuccessRepository())

        assertEquals(
            "Initial state should be Idle",
            CreateRoomUiState.Idle,
            viewModel.uiState.value
        )
    }

    // ─────────────────────────────────────────────
    // C02: 标题为空时 createRoom 返回校验错误
    // ─────────────────────────────────────────────

    @Test
    fun `C02 empty title returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())

            viewModel.createRoom(title = "", type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error for empty title, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    @Test
    fun `C02b blank title returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())

            viewModel.createRoom(title = "   ", type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error for blank title, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    // ─────────────────────────────────────────────
    // C03: 标题超过 30 字符时 createRoom 返回校验错误
    // ─────────────────────────────────────────────

    @Test
    fun `C03 title exceeding 30 chars returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())
            // 31 个 ASCII 字符
            val longTitle = "A".repeat(31)

            viewModel.createRoom(title = longTitle, type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error for title > 30 chars, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    @Test
    fun `C03b title exactly 30 chars is valid`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())
            val title30 = "A".repeat(30)

            viewModel.createRoom(title = title30, type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "Title of exactly 30 chars should succeed, but was: $state",
                state is CreateRoomUiState.Success
            )
        }

    @Test
    fun `C03c unicode title 30 chars is valid`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())
            // 30 个中文字符（Unicode，每个 = 1 字符）
            val unicodeTitle = "我".repeat(30)

            viewModel.createRoom(title = unicodeTitle, type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "30 unicode chars should succeed, but was: $state",
                state is CreateRoomUiState.Success
            )
        }

    @Test
    fun `C03d unicode title 31 chars returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())
            // 31 个中文字符
            val longUnicodeTitle = "我".repeat(31)

            viewModel.createRoom(title = longUnicodeTitle, type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "31 unicode chars should fail, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    // ─────────────────────────────────────────────
    // C04: password 类型但密码为空时 createRoom 返回校验错误
    // ─────────────────────────────────────────────

    @Test
    fun `C04 password type with empty password returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())

            viewModel.createRoom(title = "密码房间", type = "password", password = "")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error for password type with empty password, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    @Test
    fun `C04b password type with null password returns validation error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())

            viewModel.createRoom(title = "密码房间", type = "password", password = null)
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error for password type with null password, but was: $state",
                state is CreateRoomUiState.Error
            )
        }

    @Test
    fun `C04c normal type without password is valid`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val viewModel = CreateRoomViewModel(FakeSuccessRepository())

            viewModel.createRoom(title = "普通房间", type = "normal", password = null)
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "Normal type without password should succeed, but was: $state",
                state is CreateRoomUiState.Success
            )
        }

    // ─────────────────────────────────────────────
    // C05: 普通类型房间创建成功，状态变为 Success(roomId)
    // ─────────────────────────────────────────────

    @Test
    fun `C05 normal room creation success emits Success state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val expectedRoomId = "room-normal-001"
            val fakeRepo = FakeSuccessRepository(returnedRoomId = expectedRoomId)
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "我的语音房", type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Success, but was: $state",
                state is CreateRoomUiState.Success
            )
            assertEquals(
                "Success should contain the returned roomId",
                expectedRoomId,
                (state as CreateRoomUiState.Success).roomId
            )
        }

    @Test
    fun `C05b normal room creation passes correct params to repository`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeSuccessRepository()
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "测试房间", type = "normal")
            advanceUntilIdle()

            assertEquals("测试房间", fakeRepo.lastTitle)
            assertEquals("normal", fakeRepo.lastType)
            assertEquals(null, fakeRepo.lastPassword)
        }

    // ─────────────────────────────────────────────
    // C06: 密码类型房间创建成功，状态变为 Success(roomId)
    // ─────────────────────────────────────────────

    @Test
    fun `C06 password room creation success emits Success state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val expectedRoomId = "room-pwd-002"
            val fakeRepo = FakeSuccessRepository(returnedRoomId = expectedRoomId)
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "密码房间", type = "password", password = "abc123")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Success for password room, but was: $state",
                state is CreateRoomUiState.Success
            )
            assertEquals(
                "Success should contain the returned roomId",
                expectedRoomId,
                (state as CreateRoomUiState.Success).roomId
            )
        }

    @Test
    fun `C06b password room passes password to repository`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeSuccessRepository()
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "密码房间", type = "password", password = "secure123")
            advanceUntilIdle()

            assertEquals("password", fakeRepo.lastType)
            assertEquals("secure123", fakeRepo.lastPassword)
        }

    // ─────────────────────────────────────────────
    // C07: API 失败时，状态变为 Error(message)
    // ─────────────────────────────────────────────

    @Test
    fun `C07 api failure emits Error state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val errorMessage = "用户已有活跃房间"
            val fakeRepo = FakeFailureRepository(errorMessage = errorMessage)
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "新房间", type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error on API failure, but was: $state",
                state is CreateRoomUiState.Error
            )
            assertEquals(
                "Error message should match API error",
                errorMessage,
                (state as CreateRoomUiState.Error).message
            )
        }

    @Test
    fun `C07b api failure with null message emits Error state with fallback message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = object : IRoomRepository {
                override suspend fun getRooms(page: Int, size: Int) =
                    Result.success(RoomsPage(total = 0, page = 1, items = emptyList()))

                override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                    NullPagingSource()

                override suspend fun createRoom(
                    title: String,
                    type: String,
                    password: String?
                ): Result<String> = Result.failure(Exception()) // null message
            }
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "新房间", type = "normal")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "State should be Error even when exception message is null, but was: $state",
                state is CreateRoomUiState.Error
            )
            assertTrue(
                "Error message should not be empty",
                (state as CreateRoomUiState.Error).message.isNotBlank()
            )
        }

    // ─────────────────────────────────────────────
    // C08: 创建中途不能重复提交（Loading 状态下忽略二次调用）
    // ─────────────────────────────────────────────

    @Test
    fun `C08 createRoom is idempotent during Loading`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val blockingRepo = FakeBlockingRepository()
            val viewModel = CreateRoomViewModel(blockingRepo)

            // 第一次调用 — 协程入队
            viewModel.createRoom(title = "房间A", type = "normal")

            // 推进到第一次挂起点（repository.createRoom 之前设置 Loading 状态）
            mainDispatcherRule.testDispatcher.scheduler.runCurrent()

            // 此时应为 Loading 状态
            assertEquals(
                "State should be Loading after first createRoom call",
                CreateRoomUiState.Loading,
                viewModel.uiState.value
            )

            // 第二次调用 — 应被忽略（Loading 时幂等）
            viewModel.createRoom(title = "房间B", type = "normal")
            mainDispatcherRule.testDispatcher.scheduler.runCurrent()

            // 状态仍为 Loading，且仓库只被调用了一次（通过状态不变验证）
            assertEquals(
                "State should still be Loading (second call ignored)",
                CreateRoomUiState.Loading,
                viewModel.uiState.value
            )
        }

    // ─────────────────────────────────────────────
    // 额外边界测试
    // ─────────────────────────────────────────────

    @Test
    fun `extra validation error does not call repository`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeSuccessRepository()
            val viewModel = CreateRoomViewModel(fakeRepo)

            // 空标题 — 应直接返回校验错误，不调用 repository
            viewModel.createRoom(title = "", type = "normal")
            advanceUntilIdle()

            assertEquals(
                "Repository should not be called for validation errors",
                0,
                fakeRepo.createRoomCallCount
            )
        }

    @Test
    fun `extra after success state is Success with correct roomId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeSuccessRepository(returnedRoomId = "xyz-room-999")
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "Test", type = "normal")
            advanceUntilIdle()

            assertEquals(
                CreateRoomUiState.Success("xyz-room-999"),
                viewModel.uiState.value
            )
        }

    @Test
    fun `extra paid type without password is valid`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeSuccessRepository()
            val viewModel = CreateRoomViewModel(fakeRepo)

            viewModel.createRoom(title = "付费房间", type = "paid", password = null)
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue(
                "Paid type without password should succeed, but was: $state",
                state is CreateRoomUiState.Success
            )
        }

    // ─────────────────────────────────────────────
    // C09: resetState() 将 uiState 重置为 Idle
    // ─────────────────────────────────────────────

    @Test
    fun `C09 resetState resets uiState to Idle`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Arrange: 让 ViewModel 进入 Success 状态
            val fakeRepo = FakeSuccessRepository(returnedRoomId = "room-1")
            val viewModel = CreateRoomViewModel(fakeRepo)
            viewModel.createRoom(title = "测试房间", type = "normal")
            advanceUntilIdle()
            assertEquals(
                "Pre-condition: state should be Success before reset",
                CreateRoomUiState.Success("room-1"),
                viewModel.uiState.value
            )

            // Act
            viewModel.resetState()

            // Assert
            assertEquals(
                "resetState() should return uiState to Idle",
                CreateRoomUiState.Idle,
                viewModel.uiState.value
            )
        }
}
