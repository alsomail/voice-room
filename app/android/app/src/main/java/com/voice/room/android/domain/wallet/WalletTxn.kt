package com.voice.room.android.domain.wallet

/**
 * 钱包流水领域模型 (T-30027)
 *
 * @param id        流水唯一 ID
 * @param amount    金额：正数 = 收入（绿色 +），负数 = 支出（红色 -）
 * @param reason    流水原因（如 "礼物收入"、"充值"）
 * @param iconUrl   可选图标 URL
 * @param createdAt ISO-8601 创建时间
 */
data class WalletTxn(
    val id: String,
    val amount: Long,
    val reason: String,
    val iconUrl: String? = null,
    val createdAt: String,
)
