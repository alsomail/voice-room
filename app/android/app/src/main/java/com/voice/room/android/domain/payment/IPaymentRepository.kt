package com.voice.room.android.domain.payment

/**
 * 支付 Repository 契约接口 (T-30060~63)
 */
interface IPaymentRepository {
    /** GET /v1/payments/skus — 获取 SKU 列表 */
    suspend fun listSkus(): Result<List<SkuItem>>

    /** POST /v1/payments/orders — 创建订单 */
    suspend fun createOrder(skuId: String): Result<CreateOrderResult>

    /** POST /v1/payments/google/verify — 验证购买 */
    suspend fun verifyPurchase(orderId: String, purchaseToken: String): Result<VerifyResult>
}

data class CreateOrderResult(
    val orderId: String,
    val skuId: String,
    val diamonds: Long,
    val displayPriceUsd: String
)

data class VerifyResult(
    val orderId: String,
    val newState: String,
    val diamondsCredited: Long
)
