package com.voice.room.android.domain.payment

import kotlinx.coroutines.flow.SharedFlow

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

    /** 发起购买流程，结果通过 [purchaseResults] Flow 异步投递 */
    suspend fun launchBillingFlow(
        skuId: String,
        obfuscatedAccountId: String
    ): Result<Unit>

    /** 确认购买（消耗型商品） */
    suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit>

    /** 断开连接 */
    fun disconnect()

    /**
     * 购买结果异步流 — 桥接 BillingClient PurchasesUpdatedListener 回调。
     * [launchBillingFlow] 调用后，观察此 Flow 获取实际 purchaseToken。
     */
    val purchaseResults: SharedFlow<PurchaseResult>
}

data class PurchaseResult(
    val purchaseToken: String,
    val skuId: String,
    val orderId: String
)

data class ProductDetail(
    val productId: String,
    val price: String,
    val priceCurrencyCode: String,
    val priceAmountMicros: Long,
    val title: String,
    val description: String
)
