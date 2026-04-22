package com.voice.room.android.domain.ranking

/**
 * 榜单分页数据领域对象 (T-30033)
 *
 * @param type   榜单类型：charm/wealth
 * @param period 榜单周期：day/week
 * @param items  榜单条目列表（最多 50 条）
 * @param me     当前用户排名信息；未入榜时 rank=null
 */
data class RankingPage(
    val type: String,
    val period: String,
    val items: List<RankEntry>,
    val me: MyRank?,
)
