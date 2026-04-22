package com.voice.room.android.core.ws.event

/**
 * SendGift 请求的服务端响应事件 (T-30030)
 *
 * 协议格式（§6.4.2 SendGiftResult）：
 * ```json
 * { "type": "SendGiftResult", "msg_id": "uuid", "code": 0, "payload": {} }
 * ```
 *
 * @param msgId 对应客户端发送的 SendGift 请求中的 msg_id（用于幂等匹配）
 * @param code  结果码：0=成功，40290=余额不足，40400=不在房间，
 *              40402=礼物已下架，40403=接收者不在麦
 */
data class SendGiftResultEvent(
    val msgId: String,
    val code: Int,
)

/**
 * 房间广播：有人成功送礼 (T-30030 / T-30031)
 *
 * 协议格式（§6.4.3 GiftReceived）：
 * ```json
 * {
 *   "type": "GiftReceived", "msg_id": "uuid",
 *   "payload": {
 *     "gift_record_id": "uuid",
 *     "sender": { "user_id":"uuid", "nickname":"Alice", "avatar":"..." },
 *     "receiver": { "user_id":"uuid", "nickname":"Bob", "avatar":null },
 *     "gift": { "id":"uuid", "code":"castle_01", "name":"قصر",
 *               "icon_url":"...", "animation_url":"...", "effect_level":4 },
 *     "count": 1, "total_price": 520
 *   },
 *   "timestamp": 1720000000000
 * }
 * ```
 */
data class GiftReceivedEvent(
    val msgId: String,
    val giftRecordId: String,
    val senderUserId: String,
    val senderNickname: String,
    val senderAvatar: String?,
    val receiverUserId: String,
    val receiverNickname: String,
    val receiverAvatar: String?,   // 协议 §6.4.3 receiver.avatar，nullable 兼容旧协议
    val giftId: String,
    val giftCode: String,
    val giftName: String,
    val giftIconUrl: String,
    val giftAnimationUrl: String?,
    val effectLevel: Int,
    val count: Int,
    val totalPrice: Long,
    /**
     * 是否为重连补偿消息（T-30031 E31-07）。
     *
     * `true` 时仅播放 L1 弹幕，跳过 L2 麦位光圈与 L3 全屏特效。
     * 默认值 `false` 保持向后兼容。
     */
    val isReplay: Boolean = false,
)
