package com.voice.room.android.domain.ranking

/**
 * 榜单条目领域对象 (T-30033)
 *
 * @param rank     排名（1-based）
 * @param userId   用户 UUID
 * @param nickname 用户昵称
 * @param avatar   头像 URL
 * @param score    积分
 * @param medal    奖牌：gold/silver/bronze/null
 */
data class RankEntry(
    val rank: Int,
    val userId: String,
    val nickname: String,
    val avatar: String,
    val score: Long,
    val medal: String?,
)
