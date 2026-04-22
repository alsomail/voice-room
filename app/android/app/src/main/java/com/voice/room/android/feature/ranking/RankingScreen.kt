package com.voice.room.android.feature.ranking

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.feature.ranking.components.MyRankFooter
import com.voice.room.android.feature.ranking.components.RankItem

/**
 * 魅力/财富榜页 (T-30033)
 *
 * UI 结构：
 * - TopAppBar：返回键 + "榜单"标题
 * - 一级 Tab：魅力榜 / 财富榜（testTag: ranking_tab_type_charm / ranking_tab_type_wealth）
 * - 二级 Tab：日榜 / 周榜（testTag: ranking_tab_period_day / ranking_tab_period_week）
 * - 榜单列表（PullToRefreshBox）
 * - 底部固定 MyRankFooter（testTag: my_rank_footer）
 *
 * @param viewModel     由调用方注入（或通过 viewModel() 创建）
 * @param onNavigateBack 返回按钮点击回调
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RankingScreen(
    viewModel: RankingViewModel,
    onNavigateBack: () -> Unit = {},
) {
    val uiState by viewModel.uiState.collectAsState()

    Scaffold(
        containerColor = MenaColors.Background,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = "榜单",
                        color = MenaColors.Primary,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "返回",
                            tint = MenaColors.OnBackground,
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MenaColors.Surface,
                ),
            )
        },
        bottomBar = {
            MyRankFooter(myRank = uiState.myRank)
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            // ── 一级 Tab：魅力榜 / 财富榜 ─────────────────────────
            val typeValues = RankingType.entries
            TabRow(
                selectedTabIndex = typeValues.indexOf(uiState.type),
                containerColor = MenaColors.Surface,
                contentColor = MenaColors.Primary,
            ) {
                typeValues.forEach { type ->
                    Tab(
                        selected = uiState.type == type,
                        onClick = { viewModel.selectType(type) },
                        modifier = Modifier.testTag(
                            "ranking_tab_type_${type.apiValue}"
                        ),
                        text = {
                            Text(
                                text = type.displayName,
                                color = if (uiState.type == type)
                                    MenaColors.Primary
                                else MenaColors.OnBackgroundSecondary,
                            )
                        },
                    )
                }
            }

            // ── 二级 Tab：日榜 / 周榜 ──────────────────────────────
            val periodValues = Period.entries
            TabRow(
                selectedTabIndex = periodValues.indexOf(uiState.period),
                containerColor = MenaColors.SurfaceVariant,
                contentColor = MenaColors.Primary,
            ) {
                periodValues.forEach { period ->
                    Tab(
                        selected = uiState.period == period,
                        onClick = { viewModel.selectPeriod(period) },
                        modifier = Modifier.testTag(
                            "ranking_tab_period_${period.apiValue}"
                        ),
                        text = {
                            Text(
                                text = period.displayName,
                                color = if (uiState.period == period)
                                    MenaColors.Primary
                                else MenaColors.OnBackgroundSecondary,
                            )
                        },
                    )
                }
            }

            // ── 榜单内容区域 ────────────────────────────────────────
            PullToRefreshBox(
                isRefreshing = uiState.refreshing,
                onRefresh = { viewModel.refresh() },
                modifier = Modifier
                    .fillMaxSize()
                    .background(MenaColors.Background),
            ) {
                when {
                    uiState.loading -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator(
                                color = MenaColors.Primary,
                                modifier = Modifier.testTag("ranking_loading"),
                            )
                        }
                    }

                    uiState.error != null -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text(
                                    text = uiState.error ?: "加载失败",
                                    color = MenaColors.Error,
                                    modifier = Modifier.testTag("ranking_error_text"),
                                )
                                TextButton(
                                    onClick = { viewModel.refresh() },
                                    modifier = Modifier.testTag("ranking_retry_button"),
                                ) {
                                    Icon(
                                        imageVector = Icons.Filled.Refresh,
                                        contentDescription = "重试",
                                        tint = MenaColors.Primary,
                                    )
                                    Text(
                                        text = "重试",
                                        color = MenaColors.Primary,
                                    )
                                }
                            }
                        }
                    }

                    uiState.items.isEmpty() -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "暂无数据",
                                color = MenaColors.OnBackgroundSecondary,
                                fontSize = 14.sp,
                                modifier = Modifier.testTag("ranking_empty_state"),
                            )
                        }
                    }

                    else -> {
                        LazyColumn(modifier = Modifier.fillMaxSize()) {
                            items(uiState.items) { entry ->
                                RankItem(entry = entry)
                            }
                        }
                    }
                }
            }
        }
    }
}
