package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.paging.PagingSource
import androidx.paging.PagingState
import androidx.paging.compose.collectAsLazyPagingItems
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.data.room.RoomPagingSource
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — HallScreen Paging3 升级 (T-30006)
 *
 * C01: LazyPagingItems 初始加载中       → hall_loading 可见
 * C02: 数据加载完成，2 条数据             → 2 张 RoomCard 可见
 * C03: 加载失败（Error 状态）            → hall_error_text + hall_retry_button 可见
 * C04: 下拉刷新触发                      → pagingItems.refresh() 被调用
 * C05: itemCount=0, loadState=NotLoading → hall_empty_state 可见
 */
@RunWith(AndroidJUnit4::class)
class HallScreenPagingTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // Helper: 通过 ViewModelProvider 正确管理生命周期
    // ─────────────────────────────────────────────

    private fun createViewModel(repo: IRoomRepository): RoomListViewModel =
        ViewModelProvider(
            composeTestRule.activity,
            RoomListViewModel.Factory(repo)
        )[RoomListViewModel::class.java]

    // ─────────────────────────────────────────────
    // C01: 初始加载中 → hall_loading 可见
    // ─────────────────────────────────────────────

    @Test
    fun C01_initialLoading_showsLoadingIndicator() {
        val blockingRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                kotlinx.coroutines.awaitCancellation()

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                object : PagingSource<Int, RoomItem>() {
                    override fun getRefreshKey(state: PagingState<Int, RoomItem>) = null
                    override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> =
                        kotlinx.coroutines.awaitCancellation()
                }

            override suspend fun createRoom(title: String, type: String, password: String?): Result<String> =
                Result.failure(NotImplementedError())
        }
        val viewModel = createViewModel(blockingRepo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_loading").assertIsDisplayed()
        assertTrue(composeTestRule.onAllNodes(hasTestTag("room_card_id-1")).fetchSemanticsNodes().isEmpty())
    }

    // ─────────────────────────────────────────────
    // C02: 数据加载完成，2 条数据 → 2 张 RoomCard 可见
    // ─────────────────────────────────────────────

    @Test
    fun C02_dataLoaded_showsTwoRoomCards() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_card_id-1").assertIsDisplayed()
        composeTestRule.onNodeWithTag("room_card_id-2").assertIsDisplayed()
        composeTestRule.onNodeWithTag("hall_empty_state").assertIsNotDisplayed()
    }

    // ─────────────────────────────────────────────
    // C03: 加载失败 → hall_error_text 可见，重试按钮可点击
    // ─────────────────────────────────────────────

    @Test
    fun C03_loadError_showsErrorTextAndRetryButton() {
        val errorRepo = FakeRoomRepository().apply { shouldFail = true }
        val viewModel = createViewModel(errorRepo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_error_text").assertIsDisplayed()
        composeTestRule.onNodeWithTag("hall_retry_button").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // C03-ext: 点击重试按钮 → 触发 retry，重新显示内容
    // ─────────────────────────────────────────────

    @Test
    fun C03ext_clickRetryButton_isClickable() {
        val errorRepo = FakeRoomRepository().apply { shouldFail = true }
        val viewModel = createViewModel(errorRepo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        // 验证重试按钮可以被点击（不抛异常）
        composeTestRule.onNodeWithTag("hall_retry_button").performClick()
        composeTestRule.waitForIdle()
    }

    // ─────────────────────────────────────────────
    // C04: 触发 refresh → getRoomsPagingSource 被再次调用
    // ─────────────────────────────────────────────

    @Test
    fun C04_refresh_triggersNewPagingSource() {
        var pagingSourceCallCount = 0
        val trackingRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                Result.success(
                    RoomsPage(
                        total = 2, page = 1,
                        items = FakeRoomRepository().rooms
                    )
                )

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> {
                pagingSourceCallCount++
                return RoomPagingSource(this)
            }

            override suspend fun createRoom(title: String, type: String, password: String?): Result<String> =
                Result.failure(NotImplementedError())
        }
        val viewModel = createViewModel(trackingRepo)

        var capturedRefresh: (() -> Unit)? = null

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            // Capture refresh callback for programmatic trigger
            capturedRefresh = { pagingItems.refresh() }
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        val countAfterInitialLoad = pagingSourceCallCount

        // 触发 refresh（模拟下拉刷新）
        composeTestRule.runOnUiThread { capturedRefresh?.invoke() }
        composeTestRule.waitForIdle()

        assertTrue(
            "After refresh, getRoomsPagingSource should be called again. " +
                "Initial=$countAfterInitialLoad, After=${pagingSourceCallCount}",
            pagingSourceCallCount > countAfterInitialLoad
        )
    }

    // ─────────────────────────────────────────────
    // C05: 空列表 + NotLoading → hall_empty_state 可见
    // ─────────────────────────────────────────────

    @Test
    fun C05_emptyItems_showsEmptyState() {
        val emptyRepo = FakeRoomRepository().apply {
            rooms = emptyList()
            total = 0
        }
        val viewModel = createViewModel(emptyRepo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_empty_state").assertIsDisplayed()
        assertTrue(composeTestRule.onAllNodes(hasTestTag("room_card_id-1")).fetchSemanticsNodes().isEmpty())
    }
}
