package com.voice.room.android.feature.room

import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — HallViewModel
 *
 * V01: 构造后 init 触发加载，FakeRepo 返回 2 条 → rooms.size==2, isLoading==false, error==null
 * V02: 加载期间中间状态 → isLoading==true
 * V03: FakeRepo 抛 IOException → error!=null, isLoading==false, rooms.isEmpty()
 * V04: 成功加载后调用 refresh() → rooms 重新填充，currentPage 重置为 1
 * V05: total=42, page=1, size=20 → hasMore==true；total=15 时 hasMore==false
 */
@OptIn(ExperimentalCoroutinesApi::class)
class HallViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─────────────────────────────────────────────
    // V01: init 触发加载，FakeRepo 返回 2 条
    // ─────────────────────────────────────────────

    @Test
    fun `V01 init triggers load and FakeRepo 2 rooms result in rooms size 2 isLoading false error null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository()
            val viewModel = HallViewModel(fakeRepo)

            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertEquals("rooms.size should be 2", 2, state.rooms.size)
            assertFalse("isLoading should be false", state.isLoading)
            assertNull("error should be null", state.error)
        }

    // ─────────────────────────────────────────────
    // V02: 加载期间中间状态 isLoading==true
    // ─────────────────────────────────────────────

    @Test
    fun `V02 during loading isLoading is true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Use a blocking fake repo that never completes
            val blockingRepo = object : IRoomRepository {
                override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> {
                    // Suspend indefinitely to keep isLoading = true
                    kotlinx.coroutines.awaitCancellation()
                }

                override fun getRoomsPagingSource(): androidx.paging.PagingSource<Int, com.voice.room.android.domain.room.RoomItem> =
                    object :
                        androidx.paging.PagingSource<Int, com.voice.room.android.domain.room.RoomItem>() {
                        override fun getRefreshKey(state: androidx.paging.PagingState<Int, com.voice.room.android.domain.room.RoomItem>) =
                            null

                        override suspend fun load(params: androidx.paging.PagingSource.LoadParams<Int>): androidx.paging.PagingSource.LoadResult<Int, com.voice.room.android.domain.room.RoomItem> =
                            kotlinx.coroutines.awaitCancellation()
                    }

                override suspend fun createRoom(
                    title: String,
                    type: String,
                    password: String?
                ): Result<String> = kotlinx.coroutines.awaitCancellation()
            }
            val viewModel = HallViewModel(blockingRepo)

            // Run only the first update (isLoading = true), don't advance past the suspension
            mainDispatcherRule.testDispatcher.scheduler.runCurrent()

            val state = viewModel.uiState.value
            assertTrue("isLoading should be true during loading", state.isLoading)
        }

    // ─────────────────────────────────────────────
    // V03: FakeRepo 抛 IOException → error!=null, isLoading==false, rooms.isEmpty()
    // ─────────────────────────────────────────────

    @Test
    fun `V03 FakeRepo throws IOException results in error not null isLoading false rooms empty`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply { shouldFail = true }
            val viewModel = HallViewModel(fakeRepo)

            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertNotNull("error should not be null", state.error)
            assertFalse("isLoading should be false", state.isLoading)
            assertTrue("rooms should be empty", state.rooms.isEmpty())
        }

    // ─────────────────────────────────────────────
    // V04: 成功加载后调用 refresh() → rooms 重新填充，currentPage 重置为 1
    // ─────────────────────────────────────────────

    @Test
    fun `V04 refresh after success reloads rooms and resets currentPage to 1`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository()
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            // Confirm rooms loaded
            assertEquals(2, viewModel.uiState.value.rooms.size)

            // Trigger refresh
            viewModel.refresh()
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertEquals("rooms should be repopulated", 2, state.rooms.size)
            assertEquals("currentPage should be reset to 1", 1, state.currentPage)
            assertNull("error should be null after successful refresh", state.error)
        }

    // ─────────────────────────────────────────────
    // V05: hasMore logic based on total vs currentPage * PAGE_SIZE
    // ─────────────────────────────────────────────

    @Test
    fun `V05 total 42 page 1 size 20 hasMore is true total 15 hasMore is false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Case 1: total=42 > 1 * 20 → hasMore = true
            val fakeRepo42 = FakeRoomRepository().apply { total = 42 }
            val viewModel42 = HallViewModel(fakeRepo42)
            advanceUntilIdle()
            assertTrue("hasMore should be true when total=42, page=1, size=20",
                viewModel42.uiState.value.hasMore)

            // Case 2: total=15 <= 1 * 20 → hasMore = false
            val fakeRepo15 = FakeRoomRepository().apply { total = 15 }
            val viewModel15 = HallViewModel(fakeRepo15)
            advanceUntilIdle()
            assertFalse("hasMore should be false when total=15, page=1, size=20",
                viewModel15.uiState.value.hasMore)
        }

    // ─────────────────────────────────────────────
    // Extra: rooms data is correctly mapped from FakeRepo
    // ─────────────────────────────────────────────

    @Test
    fun `init loads rooms and title is correctly mapped`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository()
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            val rooms = viewModel.uiState.value.rooms
            assertEquals("id-1", rooms[0].roomId)
            assertEquals("房间A", rooms[0].title)
            assertEquals("id-2", rooms[1].roomId)
            assertEquals("房间B", rooms[1].title)
        }

    // ─────────────────────────────────────────────
    // Extra: error message is user-friendly
    // ─────────────────────────────────────────────

    @Test
    fun `error message on failure is user friendly`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository().apply { shouldFail = true }
            val viewModel = HallViewModel(fakeRepo)
            advanceUntilIdle()

            assertEquals("网络异常，请稍后重试", viewModel.uiState.value.error)
        }
}
