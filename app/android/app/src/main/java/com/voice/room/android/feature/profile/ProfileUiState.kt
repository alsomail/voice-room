package com.voice.room.android.feature.profile

import com.voice.room.android.domain.user.UserProfile

/**
 * ProfileUiState — 个人中心页面的 UI 状态（sealed interface）
 *
 * - [Loading]：正在加载，展示骨架屏/进度条
 * - [Success]：加载成功，展示用户信息；fromCache=true 时表示数据来自缓存
 * - [Error]：加载失败；cachedProfile 不为 null 时可降级展示缓存
 */
sealed interface ProfileUiState {

    /** 加载中 */
    data object Loading : ProfileUiState

    /**
     * 加载成功
     *
     * @param profile   用户资料
     * @param fromCache 是否来自本地缓存（true = 网络异常降级）
     */
    data class Success(
        val profile: UserProfile,
        val fromCache: Boolean = false,
    ) : ProfileUiState

    /**
     * 加载失败
     *
     * @param message       错误描述
     * @param cachedProfile 可选缓存数据（实际场景中 Success(fromCache=true) 已处理降级，此处备用）
     */
    data class Error(
        val message: String,
        val cachedProfile: UserProfile? = null,
    ) : ProfileUiState
}
