package com.voice.room.android.feature.ranking

/**
 * 榜单周期枚举 (T-30033)
 */
enum class Period(val apiValue: String, val displayName: String) {
    Day("day", "日榜"),
    Week("week", "周榜"),
}
