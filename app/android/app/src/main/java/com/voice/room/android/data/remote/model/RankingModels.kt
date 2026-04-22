package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 榜单条目 DTO (T-30033, ranking_api.md §1.3)
 */
data class RankEntryDto(
    @SerializedName("rank")     val rank: Int,
    @SerializedName("user_id")  val userId: String,
    @SerializedName("nickname") val nickname: String,
    @SerializedName("avatar")   val avatar: String,
    @SerializedName("score")    val score: Long,
    @SerializedName("medal")    val medal: String?,
)

/**
 * 当前用户排名 DTO (T-30033, ranking_api.md §1.3)
 *
 * rank 未入榜时为 null
 */
data class MyRankDto(
    @SerializedName("rank")  val rank: Int?,
    @SerializedName("score") val score: Long,
)

/**
 * 榜单查询响应数据体 DTO (T-30033, ranking_api.md §1.3)
 */
data class RankingDto(
    @SerializedName("type")       val type: String,
    @SerializedName("period")     val period: String,
    @SerializedName("period_key") val periodKey: String,
    @SerializedName("items")      val items: List<RankEntryDto>,
    @SerializedName("me")         val me: MyRankDto?,
)
