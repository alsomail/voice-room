package com.voice.room.android.feature.gift

/**
 * 礼物面板一次性事件 (T-30028 / T-30030)
 *
 * 通过 [GiftPanelViewModel.events] SharedFlow 发射，UI 层消费一次即丢弃。
 */
sealed class GiftPanelEvent {
    /** 点击"充值"按钮（T-30032 前占位：显示 Toast） */
    object ShowRechargeHint : GiftPanelEvent()

    /** 通用 Toast 提示 */
    data class ShowToast(val message: String) : GiftPanelEvent()

    /**
     * 余额不足弹窗（T-30030 S30-05 / T-30032）
     *
     * 触发时机：SendGiftResult code=40290（INSUFFICIENT_BALANCE）。
     * UI 层应展示 InsufficientBalanceDialog（T-30032 接入）。
     */
    object ShowInsufficientDialog : GiftPanelEvent()

    /**
     * 关闭礼物面板（T-30030 S30-14）
     *
     * 触发时机：SendGiftResult code=40400（SENDER_NOT_IN_ROOM）。
     * UI 层调用 `onDismiss()` 关闭 BottomSheet。
     */
    object DismissPanel : GiftPanelEvent()
}
