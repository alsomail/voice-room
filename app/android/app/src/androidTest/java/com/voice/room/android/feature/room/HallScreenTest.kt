package com.voice.room.android.feature.room

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.paging.PagingSource
import androidx.paging.PagingState
import androidx.paging.compose.collectAsLazyPagingItems
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.data.room.FakeRoomRepository
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 集成测试 — HallScreen（T-30005 行为 + T-30006 Paging3 升级后同步）
 *
 * C01: 2 条 rooms → 2 张 RoomCard 可见，title 文本可见
 * C02: 空列表，无错误 → hall_empty_state 可见，无 RoomCard
 * C03: 加载中（blocking source）→ hall_loading 可见，无 RoomCard
 * C04: error state → hall_error_text 可见，重试按钮存在
 * C05: 点击 RoomCard → onNavigateToRoom 以 room.roomId 被调用一次
 * B02: room_type="password" → 锁图标可见
 */
@RunWith(AndroidJUnit4::class)
class HallScreenTest {

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
    // C01: 2 RoomCards are visible with title and member count
    // ─────────────────────────────────────────────

    @Test
    fun C01_twoRooms_showsTwoRoomCards() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        // Both room cards are visible
        composeTestRule.onNodeWithTag("room_card_id-1").assertIsDisplayed()
        composeTestRule.onNodeWithTag("room_card_id-2").assertIsDisplayed()

        // Titles are visible
        composeTestRule.onNodeWithText("房间A").assertIsDisplayed()
        composeTestRule.onNodeWithText("房间B").assertIsDisplayed()

        // Member counts are visible via OnlineCountBadge (T-30022)
        composeTestRule.onNodeWithText("5").assertIsDisplayed()
        composeTestRule.onNodeWithText("10").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // C02: Empty rooms shows empty state
    // ─────────────────────────────────────────────

    @Test
    fun C02_emptyRooms_showsEmptyState() {
        val repo = FakeRoomRepository().apply {
            rooms = emptyList()
            total = 0
        }
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_empty_state").assertIsDisplayed()
        // No room cards visible
        assertTrue(
            composeTestRule.onAllNodes(hasTestTag("room_card_id-1")).fetchSemanticsNodes().isEmpty()
        )
    }

    // ─────────────────────────────────────────────
    // C03: Loading state shows progress indicator
    // ─────────────────────────────────────────────

    @Test
    fun C03_loadingState_showsProgressIndicator() {
        // Never-completing PagingSource keeps loadState.refresh = Loading
        val blockingRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                kotlinx.coroutines.awaitCancellation()

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                object : PagingSource<Int, RoomItem>() {
                    override fun getRefreshKey(state: PagingState<Int, RoomItem>) = null
                    override suspend fun load(
                        params: PagingSource.LoadParams<Int>
                    ): PagingSource.LoadResult<Int, RoomItem> =
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
        // No room cards visible
        assertTrue(
            composeTestRule.onAllNodes(hasTestTag("room_card_id-1")).fetchSemanticsNodes().isEmpty()
        )
    }

    // ─────────────────────────────────────────────
    // C04: Error state shows error text and retry button
    // ─────────────────────────────────────────────

    @Test
    fun C04_errorState_showsErrorTextAndRetryButton() {
        val repo = FakeRoomRepository().apply { shouldFail = true }
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_error_text").assertIsDisplayed()
        composeTestRule.onNodeWithTag("hall_retry_button").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // C05: Click RoomCard triggers onNavigateToRoom with roomId
    // ─────────────────────────────────────────────

    @Test
    fun C05_clickRoomCard_triggersOnNavigateToRoom() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)
        var navigatedRoomId: String? = null

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(
                pagingItems = pagingItems,
                onNavigateToRoom = { roomId -> navigatedRoomId = roomId }
            )
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_card_id-1").performClick()

        assertEquals("id-1", navigatedRoomId)
    }

    // ─────────────────────────────────────────────
    // B02: room_type="password" shows lock icon
    // ─────────────────────────────────────────────

    @Test
    fun B02_passwordRoom_showsLockIcon() {
        val repo = FakeRoomRepository() // id-2 is password type
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
            HallScreen(pagingItems = pagingItems)
        }
        composeTestRule.waitForIdle()

        // id-2 is a password room → lock icon visible
        composeTestRule.onNodeWithTag("room_type_icon_password").assertIsDisplayed()
    }
}
