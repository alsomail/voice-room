package com.voice.room.android.domain.payment

/**
 * Google Play Billing 防腐层接口 (T-30061)
 *
 * 封装 BillingClient v6+，业务层不直接 import com.android.billingclient。
 */
interface IBillingPort {
    /** 连接到 Google Play Billing 服务 */
    suspend fun connect(): Result<Unit>

    /** 查询 SKU 详情 (ProductDetails) */
    suspend fun queryProductDetails(skuIds: List<String>): Result<List<ProductDetail>>

    /** 发起购买流程，返回 purchaseToken；用户取消返回 null */
    suspend fun launchBillingFlow(
        skuId: String,
        obfuscatedAccountId: String
    ): Result<String?>

    /** 确认购买（消耗型商品） */
    suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit>

    /** 断开连接 */
    fun disconnect()
}

data class ProductDetail(
    val productId: String,
    val price: String,
    val priceCurrencyCode: String,
    val priceAmountMicros: Long,
    val title: String,
    val description: String
)
