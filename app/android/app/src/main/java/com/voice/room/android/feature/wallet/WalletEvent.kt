package com.voice.room.android.feature.wallet

/**
 * WalletEvent — 钱包页一次性事件 (T-30027)
 */
sealed class WalletEvent {

    /** 显示 Toast 消息 */
    data class ShowToast(val message: String) : WalletEvent()

    /** 跳转到登录页（401 身份过期） */
    data object NavigateToLogin : WalletEvent()

    /** 通知 UI 刷新流水 Paging 列表 */
    data object RefreshTransactions : WalletEvent()
}
