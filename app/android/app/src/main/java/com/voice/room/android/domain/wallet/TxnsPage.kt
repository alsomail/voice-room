package com.voice.room.android.domain.wallet

/**
 * 流水分页结果 (T-30027)
 *
 * @param items 当前页流水列表
 * @param total 总流水条数
 * @param page  当前页码（1-based）
 */
data class TxnsPage(
    val items: List<WalletTxn>,
    val total: Int,
    val page: Int,
)
