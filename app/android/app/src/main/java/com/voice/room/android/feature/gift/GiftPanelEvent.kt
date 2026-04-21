package com.voice.room.android.feature.gift

/**
 * 礼物面板一次性事件 (T-30028)
 *
 * 通过 [GiftPanelViewModel.events] SharedFlow 发射，UI 层消费一次即丢弃。
 */
sealed class GiftPanelEvent {
    /** 点击"充值"按钮（T-30032 前占位：显示 Toast） */
    object ShowRechargeHint : GiftPanelEvent()

    /** 通用 Toast 提示 */
    data class ShowToast(val message: String) : GiftPanelEvent()
}
