package com.voice.room.android.core.ws.event

/**
 * WS BalanceUpdated 事件数据类 (T-30027)
 *
 * 当服务端推送 `{"type":"BalanceUpdated","new_balance":xxx}` 时，
 * ViewModel 将 WS Message 解析为此类型并更新余额。
 *
 * @param newBalance 最新钻石余额
 */
data class BalanceUpdatedEvent(val newBalance: Long)
