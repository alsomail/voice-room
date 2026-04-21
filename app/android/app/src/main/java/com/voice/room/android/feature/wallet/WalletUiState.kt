package com.voice.room.android.feature.wallet

/**
 * WalletUiState — 钱包页 UI 状态 (T-30027)
 *
 * @param balance        当前钻石余额，默认 0
 * @param loadingBalance 是否正在拉取余额
 * @param refreshing     是否正在下拉刷新
 * @param error          错误描述（null 表示无错误）
 */
data class WalletUiState(
    val balance: Long = 0L,
    val loadingBalance: Boolean = true,
    val refreshing: Boolean = false,
    val error: String? = null,
)
