package com.voice.room.android.core.analytics

/**
 * Analytics 事件名常量（T-30034 / T-30035）
 *
 * 业务层使用此枚举/对象中的常量作为 [AnalyticsPort.track] 的 event 参数，
 * 避免拼写错误与散落的魔法字符串。
 *
 * T-30035 新增事件：完整核心事件埋点清单（§2.9 business_flows.md）
 */
object EventKey {
    // ─── 应用生命周期 ──────────────────────────────
    const val APP_LAUNCH = "app_launch"

    // ─── 认证 ──────────────────────────────────────
    const val LOGIN_REQUEST = "login_request"
    const val LOGIN_SUCCESS = "login_success"
    const val LOGIN_FAIL = "login_fail"
    /** 验证码登录成功（business_flows §2.9）— properties: is_new_user (Boolean) */
    const val LOGIN_VERIFY_SUCCESS = "login_verify_success"
    const val LOGOUT_CLICK = "logout_click"

    // ─── 大厅 ──────────────────────────────────────
    const val HALL_VIEW = "hall_view"
    const val ROOM_CARD_CLICK = "room_card_click"
    const val CREATE_ROOM_CLICK = "create_room_click"
    const val CREATE_ROOM_SUCCESS = "create_room_success"
    const val CREATE_ROOM_FAIL = "create_room_fail"

    // ─── 房间 ──────────────────────────────────────
    const val ROOM_ENTER = "room_enter"
    const val ROOM_LEAVE = "room_leave"

    // ─── 麦位 ──────────────────────────────────────
    const val MIC_TAKE = "mic_take"
    const val MIC_LEAVE = "mic_leave"

    // ─── 聊天 ──────────────────────────────────────
    const val CHAT_SEND = "chat_send"

    // ─── 礼物 ──────────────────────────────────────
    const val GIFT_PANEL_OPEN = "gift_panel_open"
    const val GIFT_SELECT = "gift_select"
    const val GIFT_SEND_CLICK = "gift_send_click"
    const val GIFT_SEND_SUCCESS = "gift_send_success"
    const val GIFT_SEND_FAIL = "gift_send_fail"
    const val INSUFFICIENT_BALANCE_DIALOG_SHOWN = "insufficient_balance_dialog_shown"

    // ─── 钱包 ──────────────────────────────────────
    const val WALLET_VIEW = "wallet_view"
    const val RECHARGE_CLICK = "recharge_click"

    // ─── 排行榜 ────────────────────────────────────
    const val RANKING_VIEW = "ranking_view"
    const val RANKING_TAB_SWITCH = "ranking_tab_switch"

    // ─── 个人资料 ──────────────────────────────────
    const val PROFILE_VIEW = "profile_view"

    // ─── 兼容旧常量（T-30034，保留向后兼容）──────────
    @Deprecated("Use LOGIN_SUCCESS", replaceWith = ReplaceWith("LOGIN_SUCCESS"))
    const val LOGIN_FAILED = "login_failed"
    @Deprecated("Use ROOM_ENTER", replaceWith = ReplaceWith("ROOM_ENTER"))
    const val ROOM_JOINED = "room_joined"
    @Deprecated("Use ROOM_LEAVE", replaceWith = ReplaceWith("ROOM_LEAVE"))
    const val ROOM_LEFT = "room_left"
    @Deprecated("Use CREATE_ROOM_SUCCESS", replaceWith = ReplaceWith("CREATE_ROOM_SUCCESS"))
    const val ROOM_CREATED = "room_created"
    @Deprecated("Use MIC_TAKE", replaceWith = ReplaceWith("MIC_TAKE"))
    const val MIC_TAKEN = "mic_taken"
    @Deprecated("Use MIC_LEAVE", replaceWith = ReplaceWith("MIC_LEAVE"))
    const val MIC_LEFT = "mic_left"
    @Deprecated("Use GIFT_SEND_SUCCESS", replaceWith = ReplaceWith("GIFT_SEND_SUCCESS"))
    const val GIFT_SENT = "gift_sent"
    @Deprecated("Use LOGOUT_CLICK", replaceWith = ReplaceWith("LOGOUT_CLICK"))
    const val LOGOUT = "logout"
    @Deprecated("Use WALLET_VIEW", replaceWith = ReplaceWith("WALLET_VIEW"))
    const val WALLET_VIEWED = "wallet_viewed"
    @Deprecated("Use RECHARGE_CLICK", replaceWith = ReplaceWith("RECHARGE_CLICK"))
    const val RECHARGE_INITIATED = "recharge_initiated"
}

