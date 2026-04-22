package com.voice.room.android.feature.ranking

import com.voice.room.android.domain.ranking.MyRank
import com.voice.room.android.domain.ranking.RankEntry

/**
 * 榜单页 UI 状态 (T-30033)
 *
 * @param type      当前展示的榜单类型
 * @param period    当前展示的榜单周期
 * @param items     榜单条目列表
 * @param myRank    当前用户排名信息（null=未查到/加载中）
 * @param loading   是否正在加载（初始进入时）
 * @param refreshing 是否正在下拉刷新
 * @param error     错误信息（null 表示无错误）
 */
data class RankingUiState(
    val type: RankingType = RankingType.Charm,
    val period: Period = Period.Day,
    val items: List<RankEntry> = emptyList(),
    val myRank: MyRank? = null,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val error: String? = null,
)
