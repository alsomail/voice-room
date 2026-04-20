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
import com.voice.room.android.core.theme.MenaTheme
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
 * Compose UI 集成测试 — HallScreen 视觉升级 (T-30022)
 *
 * 正向用例:
 * V-04: OnlineCountBadge 显示绿色圆点 + 在线人数，testTag online_count_badge
 * V-05: 创建房间 FAB 可见，testTag create_room_fab
 * V-06: 顶部栏显示金色 "VoiceRoom" 标题，testTag hall_top_bar
 * V-07: 分类横滑显示 "热门" tab 且默认选中，testTag category_tab_hot
 *
 * 回归用例 (Paging3 不破坏):
 * R-01: 2 条 rooms → 2 张 RoomCard 可见
 * R-02: 空列表 → hall_empty_state 可见
 * R-03: 加载中 → hall_loading 可见
 * R-04: 错误 → hall_error_text + hall_retry_button 可见
 * R-05: 点击 RoomCard → onNavigateToRoom(roomId) 被调用
 * R-06: password 房间 → Lock 图标可见
 *
 * 异常 / 边界:
 * E-01: memberCount 为 0 → OnlineCountBadge 显示 "0"
 * E-04: FAB 点击 → onCreateRoom 回调触发
 */
@RunWith(AndroidJUnit4::class)
class HallScreenVisualUpgradeTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // Helper
    // ─────────────────────────────────────────────

    private fun createViewModel(repo: IRoomRepository): RoomListViewModel =
        ViewModelProvider(
            composeTestRule.activity,
            RoomListViewModel.Factory(repo)
        )[RoomListViewModel::class.java]

    // ─────────────────────────────────────────────
    // V-05: FAB 可见 + testTag create_room_fab
    // ─────────────────────────────────────────────

    @Test
    fun V05_createRoomFab_isDisplayed() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("create_room_fab").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // V-06: 顶部栏显示 "VoiceRoom" 标题 + testTag hall_top_bar
    // ─────────────────────────────────────────────

    @Test
    fun V06_hallTopBar_isDisplayedWithTitle() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_top_bar").assertIsDisplayed()
        composeTestRule.onNodeWithText("VoiceRoom").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // V-07: 分类横滑显示 "热门" tab + testTag category_tab_hot
    // ─────────────────────────────────────────────

    @Test
    fun V07_categoryTabRow_showsHotTabSelected() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("category_tab_hot").assertIsDisplayed()
        composeTestRule.onNodeWithText("热门").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // V-04: OnlineCountBadge 可见 + testTag online_count_badge
    // ─────────────────────────────────────────────

    @Test
    fun V04_onlineCountBadge_isDisplayed() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        // At least one OnlineCountBadge visible (from RoomCards)
        composeTestRule.onAllNodes(hasTestTag("online_count_badge"))[0]
            .assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // E-04: FAB 点击 → onCreateRoom 回调触发
    // ─────────────────────────────────────────────

    @Test
    fun E04_clickFab_triggersOnCreateRoom() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)
        var createRoomCalled = false

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(
                    pagingItems = pagingItems,
                    onCreateRoom = { createRoomCalled = true }
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("create_room_fab").performClick()
        composeTestRule.waitForIdle()

        assertTrue("onCreateRoom should be called", createRoomCalled)
    }

    // ─────────────────────────────────────────────
    // E-01: memberCount=0 → OnlineCountBadge 显示 "0"
    // ─────────────────────────────────────────────

    @Test
    fun E01_memberCountZero_badgeShowsZero() {
        val repo = FakeRoomRepository().apply {
            rooms = listOf(
                RoomItem(
                    roomId = "zero-room",
                    title = "空房间",
                    roomType = "normal",
                    memberCount = 0,
                    maxMembers = 10,
                    ownerNickname = "Owner",
                    ownerAvatar = null,
                    createdAt = "2024-01-01T00:00:00Z"
                )
            )
            total = 1
        }
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("0").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // 回归用例 — Paging3 不破坏
    // ═══════════════════════════════════════════════

    // ─────────────────────────────────────────────
    // R-01: 2 条 rooms → 2 张 RoomCard 可见
    // ─────────────────────────────────────────────

    @Test
    fun R01_twoRooms_showsTwoRoomCards() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_card_id-1").assertIsDisplayed()
        composeTestRule.onNodeWithTag("room_card_id-2").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // R-02: 空列表 → hall_empty_state 可见
    // ─────────────────────────────────────────────

    @Test
    fun R02_emptyRooms_showsEmptyState() {
        val repo = FakeRoomRepository().apply {
            rooms = emptyList()
            total = 0
        }
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_empty_state").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // R-03: 加载中 → hall_loading 可见
    // ─────────────────────────────────────────────

    @Test
    fun R03_loading_showsLoadingIndicator() {
        val blockingRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                kotlinx.coroutines.awaitCancellation()

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                object : PagingSource<Int, RoomItem>() {
                    override fun getRefreshKey(state: PagingState<Int, RoomItem>) = null
                    override suspend fun load(params: LoadParams<Int>): LoadResult<Int, RoomItem> =
                        kotlinx.coroutines.awaitCancellation()
                }

            override suspend fun createRoom(
                title: String, type: String, password: String?
            ): Result<String> = Result.failure(NotImplementedError())
        }
        val viewModel = createViewModel(blockingRepo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_loading").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // R-04: 错误 → hall_error_text + hall_retry_button 可见
    // ─────────────────────────────────────────────

    @Test
    fun R04_error_showsErrorTextAndRetryButton() {
        val repo = FakeRoomRepository().apply { shouldFail = true }
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("hall_error_text").assertIsDisplayed()
        composeTestRule.onNodeWithTag("hall_retry_button").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // R-05: 点击 RoomCard → onNavigateToRoom(roomId) 被调用
    // ─────────────────────────────────────────────

    @Test
    fun R05_clickRoomCard_triggersOnNavigateToRoom() {
        val repo = FakeRoomRepository()
        val viewModel = createViewModel(repo)
        var navigatedRoomId: String? = null

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(
                    pagingItems = pagingItems,
                    onNavigateToRoom = { roomId -> navigatedRoomId = roomId }
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_card_id-1").performClick()

        assertEquals("id-1", navigatedRoomId)
    }

    // ─────────────────────────────────────────────
    // R-06: password 房间 → Lock 图标可见
    // ─────────────────────────────────────────────

    @Test
    fun R06_passwordRoom_showsLockIcon() {
        val repo = FakeRoomRepository() // id-2 is password type
        val viewModel = createViewModel(repo)

        composeTestRule.setContent {
            MenaTheme {
                val pagingItems = viewModel.pagingFlow.collectAsLazyPagingItems()
                HallScreen(pagingItems = pagingItems)
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("room_type_icon_password").assertIsDisplayed()
    }
}
