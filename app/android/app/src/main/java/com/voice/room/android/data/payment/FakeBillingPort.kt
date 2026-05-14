package com.voice.room.android.data.payment

import com.voice.room.android.domain.payment.IBillingPort
import com.voice.room.android.domain.payment.ProductDetail

/**
 * Fake BillingPort — 单测用 (T-30061)
 */
class FakeBillingPort(
    private val successOnSkuIds: Set<String> = emptySet(),
    private val failMode: BillingFailMode = BillingFailMode.NONE
) : IBillingPort {

    var purchaseLaunched = false
    var lastLaunchedSku: String? = null

    override suspend fun connect(): Result<Unit> {
        if (failMode == BillingFailMode.CONNECT) return Result.failure(RuntimeException("Billing unavailable"))
        return Result.success(Unit)
    }

    override suspend fun queryProductDetails(skuIds: List<String>): Result<List<ProductDetail>> {
        return Result.success(skuIds.map { id ->
            ProductDetail(
                productId = id,
                price = "$4.99",
                priceCurrencyCode = "USD",
                priceAmountMicros = 4_990_000,
                title = "Fake SKU $id",
                description = "Test product"
            )
        })
    }

    override suspend fun launchBillingFlow(
        skuId: String,
        obfuscatedAccountId: String
    ): Result<String?> {
        purchaseLaunched = true
        lastLaunchedSku = skuId
        return when {
            failMode == BillingFailMode.USER_CANCEL -> Result.success(null)
            failMode == BillingFailMode.PURCHASE_FAIL -> Result.failure(RuntimeException("Billing error"))
            else -> Result.success("fake_purchase_token_$skuId")
        }
    }

    override suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit> {
        return Result.success(Unit)
    }

    override fun disconnect() {}

    enum class BillingFailMode { NONE, CONNECT, USER_CANCEL, PURCHASE_FAIL }
}
