package com.voice.room.android.feature.ranking

/**
 * 榜单页一次性事件 (T-30033)
 */
sealed class RankingEvent {
    /** 401 未授权，跳转登录页 */
    object NavigateToLogin : RankingEvent()
}
