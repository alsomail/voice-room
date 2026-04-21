package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 钱包余额 DTO (T-30027)
 */
data class BalanceDto(
    @SerializedName("balance") val balance: Long,
)

/**
 * 单条流水 DTO (T-30027)
 */
data class TxnDto(
    @SerializedName("id") val id: String,
    @SerializedName("amount") val amount: Long,
    @SerializedName("reason") val reason: String,
    @SerializedName("icon_url") val iconUrl: String? = null,
    @SerializedName("created_at") val createdAt: String,
)

/**
 * 通用分页包装 DTO (T-30027)
 *
 * @param T 列表项类型
 */
data class PageDto<T>(
    @SerializedName("items") val items: List<T>,
    @SerializedName("total") val total: Int,
    @SerializedName("page") val page: Int,
    @SerializedName("size") val size: Int,
)
