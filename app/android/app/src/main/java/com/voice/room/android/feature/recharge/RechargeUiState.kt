package com.voice.room.android.feature.recharge

import com.voice.room.android.domain.payment.SkuItem

data class RechargeUiState(
    val balance: Long = 0,
    val skus: List<SkuItem> = emptyList(),
    val selectedSku: SkuItem? = null,
    val isLoadingSkus: Boolean = false,
    val isCreatingOrder: Boolean = false,
    val error: String? = null,
    val orderCreated: Boolean = false,
    val createdOrderId: String? = null
)
