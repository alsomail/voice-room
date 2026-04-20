package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.paging.LoadState
import androidx.paging.compose.LazyPagingItems
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTypography
import com.voice.room.android.domain.room.RoomItem

/**
 * 大厅页 — 黑金视觉升级版 (T-30022)
 *
 * 视觉改造:
 * - Scaffold 包裹：topBar=[HallTopBar]、FAB=[CreateRoomFab]、containerColor=[MenaColors.Background]
 * - 分类横滑 [CategoryTabRow]（占位，仅"热门"可交互）
 * - 深色 [RoomCard] + [OnlineCountBadge]
 * - 所有颜色取 [MenaColors]
 *
 * Paging3 逻辑完全不变:
 * - 下拉刷新（[PullToRefreshBox]）
 * - 初始加载 / 刷新加载指示器（`hall_loading`）
 * - 加载失败重试（`hall_error_text` + `hall_retry_button`）
 * - 空列表状态（`hall_empty_state`）
 * - 上拉加载更多指示器（`hall_loading_more`）
 *
 * @param pagingItems        由 [RoomListViewModel.pagingFlow.collectAsLazyPagingItems] 提供
 * @param onNavigateToRoom   点击房间卡片时调用，参数为 roomId
 * @param onCreateRoom       FAB 点击回调（触发创建房间 BottomSheet）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HallScreen(
    pagingItems: LazyPagingItems<RoomItem>,
    onNavigateToRoom: (String) -> Unit = {},
    onCreateRoom: () -> Unit = {},
) {
    Scaffold(
        containerColor = MenaColors.Background,
        topBar = { HallTopBar() },
        floatingActionButton = { CreateRoomFab(onClick = onCreateRoom) },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            // 分类横滑占位
            CategoryTabRow()

            // ── Paging3 内容区（逻辑不变） ──────────────
            val isRefreshing = pagingItems.loadState.refresh is LoadState.Loading

            PullToRefreshBox(
                isRefreshing = isRefreshing,
                onRefresh = { pagingItems.refresh() },
                modifier = Modifier.fillMaxSize(),
            ) {
                when (val refresh = pagingItems.loadState.refresh) {

                    is LoadState.Loading -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator(
                                modifier = Modifier.testTag("hall_loading"),
                                color = MenaColors.Primary,
                            )
                        }
                    }

                    is LoadState.Error -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.spacedBy(16.dp),
                            ) {
                                Text(
                                    text = refresh.error.message ?: "加载失败",
                                    style = MenaTypography.bodyMedium,
                                    color = MenaColors.Error,
                                    modifier = Modifier.testTag("hall_error_text"),
                                )
                                Button(
                                    onClick = { pagingItems.retry() },
                                    modifier = Modifier.testTag("hall_retry_button"),
                                    colors = ButtonDefaults.buttonColors(
                                        containerColor = MenaColors.Primary,
                                        contentColor = MenaColors.OnBackground,
                                    ),
                                ) {
                                    Text("重试")
                                }
                            }
                        }
                    }

                    is LoadState.NotLoading -> {
                        if (pagingItems.itemCount == 0) {
                            Box(
                                modifier = Modifier.fillMaxSize(),
                                contentAlignment = Alignment.Center,
                            ) {
                                Text(
                                    text = "暂无房间",
                                    style = MenaTypography.bodyMedium,
                                    color = MenaColors.OnBackgroundSecondary,
                                    modifier = Modifier.testTag("hall_empty_state"),
                                )
                            }
                        } else {
                            LazyVerticalGrid(
                                columns = GridCells.Fixed(2),
                                modifier = Modifier
                                    .fillMaxSize()
                                    .padding(horizontal = 16.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                                verticalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                items(
                                    count = pagingItems.itemCount,
                                    key = { index -> pagingItems.peek(index)?.roomId ?: index },
                                ) { index ->
                                    pagingItems[index]?.let { room ->
                                        RoomCard(
                                            room = room,
                                            onClick = { onNavigateToRoom(room.roomId) },
                                        )
                                    }
                                }
                                if (pagingItems.loadState.append is LoadState.Loading) {
                                    item {
                                        CircularProgressIndicator(
                                            modifier = Modifier.testTag("hall_loading_more"),
                                            color = MenaColors.Primary,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/**
 * 创建房间 FAB — 金色 + 白色加号图标 (T-30022)
 *
 * @param onClick 点击回调
 */
@Composable
private fun CreateRoomFab(onClick: () -> Unit) {
    FloatingActionButton(
        onClick = onClick,
        containerColor = MenaColors.Primary,
        contentColor = MenaColors.OnBackground,
        modifier = Modifier.testTag("create_room_fab"),
    ) {
        Icon(
            imageVector = Icons.Default.Add,
            contentDescription = "创建房间",
        )
    }
}
