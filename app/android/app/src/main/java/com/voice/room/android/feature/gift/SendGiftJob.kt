package com.voice.room.android.feature.gift

/**
 * 单次发送礼物作业描述 (T-30030)
 *
 * 包含发送一次 SendGift WS 信令所需的全部字段。
 * [msgId] 由 [ComboAggregator] 按连击窗口生成，保证幂等性。
 *
 * @param msgId       客户端生成的 UUID，用于请求/响应匹配和服务端幂等去重
 * @param giftId      礼物 ID
 * @param recipientId 接收者用户 ID（当前在麦位上的用户）
 * @param count       赠送数量（连击聚合后的累计数量）
 * @param roomId      当前所在房间 ID
 * @param startTs     任务创建时间戳（毫秒），用于超时计算与日志
 */
data class SendGiftJob(
    val msgId: String,
    val giftId: String,
    val recipientId: String,
    val count: Int,
    val roomId: String,
    val startTs: Long = System.currentTimeMillis(),
)
