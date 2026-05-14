package com.voice.room.android.data.payment

import com.voice.room.android.domain.payment.IBillingPort
import com.voice.room.android.domain.payment.ProductDetail
import com.voice.room.android.domain.payment.PurchaseResult
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * Fake BillingPort — 单测用 (T-30061)
 */
class FakeBillingPort(
    private val successOnSkuIds: Set<String> = emptySet(),
    private val failMode: BillingFailMode = BillingFailMode.NONE
) : IBillingPort {

    var purchaseLaunched = false
    var lastLaunchedSku: String? = null

    private val _purchaseResults = MutableSharedFlow<PurchaseResult>(extraBufferCapacity = 8)
    override val purchaseResults: SharedFlow<PurchaseResult> = _purchaseResults.asSharedFlow()

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
    ): Result<Unit> {
        purchaseLaunched = true
        lastLaunchedSku = skuId
        return when {
            failMode == BillingFailMode.USER_CANCEL -> Result.failure(RuntimeException("User cancelled"))
            failMode == BillingFailMode.PURCHASE_FAIL -> Result.failure(RuntimeException("Billing error"))
            else -> {
                _purchaseResults.tryEmit(
                    PurchaseResult("fake_purchase_token_$skuId", skuId, obfuscatedAccountId)
                )
                Result.success(Unit)
            }
        }
    }

    override suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit> {
        return Result.success(Unit)
    }

    override fun disconnect() {}

    enum class BillingFailMode { NONE, CONNECT, USER_CANCEL, PURCHASE_FAIL }
}
