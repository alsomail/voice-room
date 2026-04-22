package com.voice.room.android.domain.ranking

/**
 * 当前用户排名领域对象 (T-30033)
 *
 * @param rank  当前用户排名（1-based）；未入榜为 null
 * @param score 当前用户积分；未入榜为 0
 */
data class MyRank(
    val rank: Int?,
    val score: Long,
)
