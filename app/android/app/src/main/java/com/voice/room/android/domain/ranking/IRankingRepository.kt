package com.voice.room.android.domain.ranking

/**
 * 榜单 Repository 接口 (T-30033)
 *
 * 实现：[com.voice.room.android.data.ranking.RetrofitRankingRepository]
 */
interface IRankingRepository {

    /**
     * 查询榜单
     *
     * @param type   榜单类型：`charm`（魅力榜）/ `wealth`（财富榜）
     * @param period 榜单周期：`day`（日榜）/ `week`（周榜）
     * @return [RankingPage] 成功；[Result.Failure] 包含 [com.voice.room.android.data.auth.ApiException]
     */
    suspend fun getRanking(type: String, period: String): Result<RankingPage>
}
