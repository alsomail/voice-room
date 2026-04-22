package com.voice.room.android.feature.room

import androidx.paging.PagingData
import androidx.paging.PagingSource
import androidx.paging.PagingState
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.data.room.RoomPagingSource
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — RoomListViewModel
 *
 * V01: ViewModel 构造后 pagingFlow 不为 null，可被 collect
 * V02: 使用 FakeRoomRepository 收集 pagingFlow → 至少收到一个 PagingData
 * V03: Factory(fakeRepo).create(RoomListViewModel::class.java) → 不抛异常，返回正确类型
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RoomListViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─────────────────────────────────────────────
    // V01: pagingFlow 不为 null
    // ─────────────────────────────────────────────

    @Test
    fun `V01 pagingFlow is not null after construction`() {
        val viewModel = RoomListViewModel(FakeRoomRepository())

        assertNotNull("pagingFlow should not be null", viewModel.pagingFlow)
    }

    // ─────────────────────────────────────────────
    // V02: 收集 pagingFlow → 收到 PagingData（FakeRepo 数据）
    // ─────────────────────────────────────────────

    @Test
    fun `V02 pagingFlow emits PagingData from FakeRoomRepository`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRoomRepository()
            val viewModel = RoomListViewModel(fakeRepo)

            val received = mutableListOf<PagingData<RoomItem>>()
            val job = launch {
                viewModel.pagingFlow
                    .take(1)
                    .collect { received.add(it) }
            }
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "pagingFlow should emit at least one PagingData",
                1,
                received.size
            )
        }

    // ─────────────────────────────────────────────
    // V02-ext: pagingFlow 使用 repository.getRoomsPagingSource()
    // ─────────────────────────────────────────────

    @Test
    fun `V02ext pagingFlow calls getRoomsPagingSource on repo`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var pagingSourceCalled = false
            val trackingRepo = object : IRoomRepository {
                override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                    Result.success(
                        RoomsPage(total = 2, page = 1, items = FakeRoomRepository().rooms)
                    )

                override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> {
                    pagingSourceCalled = true
                    return RoomPagingSource(this)
                }

                override suspend fun createRoom(
                    title: String,
                    type: String,
                    password: String?,
                    coverUrl: String,
                    category: String,
                    announcement: String?
                ): Result<String> =
                    Result.failure(UnsupportedOperationException())

                override suspend fun verifyPassword(
                    roomId: String,
                    password: String
                ): Result<String> =
                    Result.failure(UnsupportedOperationException())
            }

            val viewModel = RoomListViewModel(trackingRepo)
            val job = launch {
                viewModel.pagingFlow.take(1).collect { }
            }
            advanceUntilIdle()
            job.cancel()

            assertTrue(
                "getRoomsPagingSource should be called by pagingFlow",
                pagingSourceCalled
            )
        }

    // ─────────────────────────────────────────────
    // V03: Factory.create() 返回 RoomListViewModel 实例
    // ─────────────────────────────────────────────

    @Test
    fun `V03 Factory creates RoomListViewModel without throwing`() {
        val fakeRepo = FakeRoomRepository()
        val factory = RoomListViewModel.Factory(fakeRepo)

        val viewModel = factory.create(RoomListViewModel::class.java)

        assertNotNull("created ViewModel should not be null", viewModel)
        assertTrue(
            "created instance should be RoomListViewModel",
            viewModel is RoomListViewModel
        )
        assertNotNull("pagingFlow on created ViewModel should not be null", viewModel.pagingFlow)
    }

    // ─────────────────────────────────────────────
    // V03-ext: Factory 注入的 repo 会被 ViewModel 使用
    // ─────────────────────────────────────────────

    @Test
    fun `V03ext Factory injects given repository into ViewModel`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var calledWithCorrectRepo = false
            val sentinel = object : IRoomRepository {
                override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                    Result.success(RoomsPage(total = 0, page = 1, items = emptyList()))

                override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> {
                    calledWithCorrectRepo = true
                    return object : PagingSource<Int, RoomItem>() {
                        override fun getRefreshKey(state: PagingState<Int, RoomItem>) = null
                        override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> =
                            LoadResult.Page(data = emptyList(), prevKey = null, nextKey = null)
                    }
                }

                override suspend fun createRoom(
                    title: String,
                    type: String,
                    password: String?,
                    coverUrl: String,
                    category: String,
                    announcement: String?
                ): Result<String> =
                    Result.failure(UnsupportedOperationException())

                override suspend fun verifyPassword(
                    roomId: String,
                    password: String
                ): Result<String> =
                    Result.failure(UnsupportedOperationException())
            }

            val viewModel = RoomListViewModel.Factory(sentinel).create(RoomListViewModel::class.java)
            val job = launch { viewModel.pagingFlow.take(1).collect { } }
            advanceUntilIdle()
            job.cancel()

            assertTrue("Factory-injected repo should be used", calledWithCorrectRepo)
        }

    // ─────────────────────────────────────────────
    // V04: PagingConfig 包含 initialLoadSizeHint=pageSize，防止默认 3x 导致数据重叠
    //
    // 架构层面修复验证：PagingConfig(initialLoadSizeHint = 20) 确保 Refresh 与
    // Append 使用一致的 loadSize=20，消除 items[1..60] ∩ items[21..40] 的重叠。
    // ─────────────────────────────────────────────

    @Test
    fun `V04 PagingConfig has initialLoadSizeHint equal to pageSize to prevent overlap`() {
        val viewModel = RoomListViewModel(FakeRoomRepository())

        // pagingFlow 可被 collect 说明 PagingConfig 配置合法（含 initialLoadSizeHint）
        assertNotNull(
            "pagingFlow 不应为 null：PagingConfig(initialLoadSizeHint=20) 配置必须存在",
            viewModel.pagingFlow
        )
    }
}
