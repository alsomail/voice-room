package com.voice.room.android.feature.ranking

/**
 * 榜单类型枚举 (T-30033)
 */
enum class RankingType(val apiValue: String, val displayName: String) {
    Charm("charm", "魅力榜"),
    Wealth("wealth", "财富榜"),
}
