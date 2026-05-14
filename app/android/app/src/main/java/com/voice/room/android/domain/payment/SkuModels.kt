package com.voice.room.android.domain.payment

/**
 * SKU 领域模型 (T-30060)
 */
data class SkuItem(
    val skuId: String,
    val provider: String,
    val diamonds: Long,
    val displayPriceUsd: String,
    val displayPriceLocal: String?,
    val displayCurrency: String?,
    val isActive: Boolean,
    val sortOrder: Int,
    val tag: String?
)

/** Order state enum matching App Server payment_orders.state */
enum class OrderState(val value: String) {
    PENDING("PENDING"),
    VERIFYING("VERIFYING"),
    VERIFIED("VERIFIED"),
    CREDITED("CREDITED"),
    ACKED("ACKED"),
    CANCELLED("CANCELLED"),
    FAILED("FAILED"),
    REFUNDED("REFUNDED");

    companion object {
        fun fromValue(v: String): OrderState =
            entries.find { it.value.equals(v, ignoreCase = true) } ?: PENDING
    }
}
