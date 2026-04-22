package com.voice.room.android.core.analytics

/**
 * Analytics 事件名常量（T-30034）
 *
 * 业务层使用此枚举/对象中的常量作为 [AnalyticsPort.track] 的 event 参数，
 * 避免拼写错误与散落的魔法字符串。
 */
object EventKey {
    // ─── 房间 ───────────────────────────────────
    const val ROOM_JOINED = "room_joined"
    const val ROOM_LEFT = "room_left"
    const val ROOM_CREATED = "room_created"

    // ─── 麦位 ───────────────────────────────────
    const val MIC_TAKEN = "mic_taken"
    const val MIC_LEFT = "mic_left"

    // ─── 礼物 ───────────────────────────────────
    const val GIFT_SENT = "gift_sent"

    // ─── 认证 ───────────────────────────────────
    const val LOGIN_SUCCESS = "login_success"
    const val LOGIN_FAILED = "login_failed"
    const val LOGOUT = "logout"

    // ─── 钱包 ───────────────────────────────────
    const val WALLET_VIEWED = "wallet_viewed"
    const val RECHARGE_INITIATED = "recharge_initiated"
}
