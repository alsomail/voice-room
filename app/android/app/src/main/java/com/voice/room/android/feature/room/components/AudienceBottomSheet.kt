package com.voice.room.android.feature.room.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.data.model.RoomMember
import com.voice.room.android.feature.room.AudienceUiState

/**
 * 观众席 ModalBottomSheet（T-30039）
 *
 * 展示房间内所有成员：麦上置顶 + 观众。
 * 支持分页（滚动到底自动触发 [onLoadMore]）和实时 WS 更新。
 *
 * ### testTag
 * - 整体 Sheet：`audience_sheet`
 * - 麦上区 Header：`audience_header_on_mic`
 * - 观众区 Header：`audience_header_observers`
 * - 每行成员：`audience_item_${userId}`（由 [MemberRow] 设置）
 *
 * @param state         观众席 UI 状态（由 RoomViewModel.audienceState 驱动）
 * @param onDismiss     关闭 Sheet 的回调
 * @param onMemberClick 点击成员行回调（打开 UserActionBottomSheet，T-30040）
 * @param onLoadMore    滚动到底时触发加载更多
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AudienceBottomSheet(
    state: AudienceUiState,
    onDismiss: () -> Unit,
    onMemberClick: (RoomMember) -> Unit,
    onLoadMore: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val sheetState = rememberModalBottomSheetState(
        skipPartiallyExpanded = false,
    )
    val listState = rememberLazyListState()

    // 检测是否滚到底部，触发分页加载
    val shouldLoadMore = remember {
        derivedStateOf {
            val layoutInfo = listState.layoutInfo
            val lastVisible = layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            val totalItems = layoutInfo.totalItemsCount
            totalItems > 0 && lastVisible >= totalItems - 3
        }
    }

    LaunchedEffect(shouldLoadMore.value) {
        if (shouldLoadMore.value && state.hasMore && !state.loading) {
            onLoadMore()
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = modifier.testTag("audience_sheet"),
    ) {
        Column(modifier = Modifier.fillMaxWidth()) {
            // Header：观众席标题
            Text(
                text = "观众席 (${state.total})",
                fontSize = 18.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
            )

            HorizontalDivider()

            LazyColumn(
                state = listState,
                modifier = Modifier.fillMaxWidth(),
            ) {
                // ─── 麦上区 ──────────────────────────────────────────────
                if (state.onMic.isNotEmpty()) {
                    item(key = "header_on_mic") {
                        SectionHeader(
                            text = "麦上 (${state.onMic.size})",
                            testTag = "audience_header_on_mic",
                        )
                    }
                    items(
                        items = state.onMic,
                        key = { it.id },
                    ) { member ->
                        MemberRow(member = member, onClick = onMemberClick)
                    }
                }

                // ─── 观众区 ──────────────────────────────────────────────
                item(key = "header_observers") {
                    SectionHeader(
                        text = "观众 (${state.audience.size})",
                        testTag = "audience_header_observers",
                    )
                }
                items(
                    items = state.audience,
                    key = { it.id },
                ) { member ->
                    MemberRow(member = member, onClick = onMemberClick)
                }

                // 空状态文案
                if (state.onMic.isEmpty() && state.audience.isEmpty()) {
                    item(key = "empty") {
                        Text(
                            text = "暂无成员",
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(24.dp),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SectionHeader(
    text: String,
    testTag: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text,
        fontSize = 14.sp,
        fontWeight = FontWeight.SemiBold,
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .testTag(testTag),
    )
}
