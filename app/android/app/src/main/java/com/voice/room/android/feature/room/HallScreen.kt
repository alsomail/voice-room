package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.paging.LoadState
import androidx.paging.compose.LazyPagingItems
import coil.compose.AsyncImage
import com.voice.room.android.R
import com.voice.room.android.domain.room.RoomItem

/**
 * 大厅页 — Paging3 无限滚动版本 (T-30006 升级)
 *
 * 支持：
 * - 下拉刷新（[PullToRefreshBox]）
 * - 初始加载 / 刷新加载指示器（`hall_loading`）
 * - 加载失败重试（`hall_error_text` + `hall_retry_button`）
 * - 空列表状态（`hall_empty_state`）
 * - 上拉加载更多指示器（`hall_loading_more`）
 *
 * 与 [HallViewModel]（T-30005）并列，各自独立，不互相干扰。
 *
 * @param pagingItems        由 [RoomListViewModel.pagingFlow.collectAsLazyPagingItems] 提供
 * @param onNavigateToRoom   点击房间卡片时调用，参数为 roomId（T-30007 正式导航接入）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HallScreen(
    pagingItems: LazyPagingItems<RoomItem>,
    onNavigateToRoom: (String) -> Unit = {}
) {
    val isRefreshing = pagingItems.loadState.refresh is LoadState.Loading

    PullToRefreshBox(
        isRefreshing = isRefreshing,
        onRefresh = { pagingItems.refresh() },
        modifier = Modifier.fillMaxSize()
    ) {
        when (val refresh = pagingItems.loadState.refresh) {

            is LoadState.Loading -> {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator(
                        modifier = Modifier.testTag("hall_loading")
                    )
                }
            }

            is LoadState.Error -> {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        Text(
                            text = refresh.error.message ?: "加载失败",
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("hall_error_text")
                        )
                        Button(
                            onClick = { pagingItems.retry() },
                            modifier = Modifier.testTag("hall_retry_button")
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
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "暂无房间",
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.testTag("hall_empty_state")
                        )
                    }
                } else {
                    LazyVerticalGrid(
                        columns = GridCells.Adaptive(160.dp),
                        modifier = Modifier.fillMaxSize()
                    ) {
                        items(
                            count = pagingItems.itemCount,
                            key = { index -> pagingItems.peek(index)?.roomId ?: index }
                        ) { index ->
                            pagingItems[index]?.let { room ->
                                RoomCard(
                                    room = room,
                                    onClick = { onNavigateToRoom(room.roomId) }
                                )
                            }
                        }
                        if (pagingItems.loadState.append is LoadState.Loading) {
                            item {
                                CircularProgressIndicator(
                                    modifier = Modifier.testTag("hall_loading_more")
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

/**
 * 单个房间卡片（保持不变，T-30005 实现）
 *
 * 展示：房主头像（Coil 异步加载）、房间标题、在线人数、房主昵称、密码房锁形图标
 *
 * @param room    [RoomItem] 领域模型
 * @param onClick 点击回调（触发进房导航）
 */
@Composable
fun RoomCard(
    room: RoomItem,
    onClick: () -> Unit
) {
    Card(
        onClick = onClick,
        modifier = Modifier
            .padding(8.dp)
            .fillMaxWidth()
            .testTag("room_card_${room.roomId}")
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            AsyncImage(
                model = room.ownerAvatar,
                contentDescription = null,
                placeholder = painterResource(R.drawable.ic_placeholder),
                fallback = painterResource(R.drawable.ic_placeholder),
                modifier = Modifier.size(48.dp)
            )
            Text(
                text = room.title,
                style = MaterialTheme.typography.titleSmall
            )
            Text(
                text = "${room.memberCount}/${room.maxMembers}",
                style = MaterialTheme.typography.bodySmall
            )
            Text(
                text = room.ownerNickname,
                style = MaterialTheme.typography.bodySmall
            )
            if (room.roomType == "password") {
                Icon(
                    imageVector = Icons.Default.Lock,
                    contentDescription = null,
                    modifier = Modifier.testTag("room_type_icon_password")
                )
            }
        }
    }
}
