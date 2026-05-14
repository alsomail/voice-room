package com.voice.room.android.data.payment

import android.app.Activity
import android.content.Context
import com.android.billingclient.api.*
import com.voice.room.android.domain.payment.IBillingPort
import com.voice.room.android.domain.payment.ProductDetail
import com.voice.room.android.domain.payment.PurchaseResult
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume

class GooglePlayBillingAdapter(
    private val context: Context
) : IBillingPort {

    private var billingClient: BillingClient? = null
    private var lastLaunchedSkuId: String? = null
    private var lastLaunchedOrderId: String? = null

    private val _purchaseResults = MutableSharedFlow<PurchaseResult>(extraBufferCapacity = 8)
    override val purchaseResults: SharedFlow<PurchaseResult> = _purchaseResults.asSharedFlow()

    override suspend fun connect(): Result<Unit> {
        val client = BillingClient.newBuilder(context)
            .setListener { _, purchases ->
                purchases?.forEach { purchase ->
                    if (purchase.purchaseState == Purchase.PurchaseState.PURCHASED) {
                        val skuId = lastLaunchedSkuId ?: purchase.skus.firstOrNull() ?: return@forEach
                        val orderId = lastLaunchedOrderId ?: return@forEach
                        _purchaseResults.tryEmit(
                            PurchaseResult(
                                purchaseToken = purchase.purchaseToken,
                                skuId = skuId,
                                orderId = orderId
                            )
                        )
                    }
                }
            }
            .enablePendingPurchases()
            .build()
        billingClient = client

        return suspendCancellableCoroutine { cont ->
            client.startConnection(object : BillingClientStateListener {
                override fun onBillingSetupFinished(billingResult: BillingResult) {
                    if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                        cont.resume(Result.success(Unit))
                    } else {
                        cont.resume(Result.failure(
                            RuntimeException("Billing setup failed: ${billingResult.responseCode}")
                        ))
                    }
                }
                override fun onBillingServiceDisconnected() {}
            })
        }
    }

    override suspend fun queryProductDetails(skuIds: List<String>): Result<List<ProductDetail>> {
        val client = billingClient
            ?: return Result.failure(IllegalStateException("BillingClient not connected"))
        val productList = skuIds.map { id ->
            QueryProductDetailsParams.Product.newBuilder()
                .setProductId(id).setProductType(BillingClient.ProductType.INAPP).build()
        }
        val params = QueryProductDetailsParams.newBuilder().setProductList(productList).build()
        return suspendCancellableCoroutine { cont ->
            client.queryProductDetailsAsync(params) { billingResult, productDetailsList ->
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                    cont.resume(Result.success(productDetailsList.orEmpty().map { pd ->
                        val price = pd.oneTimePurchaseOfferDetails
                        ProductDetail(pd.productId, price?.formattedPrice ?: "",
                            price?.priceCurrencyCode ?: "", price?.priceAmountMicros ?: 0L,
                            pd.title, pd.description)
                    }))
                } else {
                    cont.resume(Result.failure(RuntimeException("queryProductDetails failed: ${billingResult.responseCode}")))
                }
            }
        }
    }

    override suspend fun launchBillingFlow(skuId: String, obfuscatedAccountId: String): Result<Unit> {
        val client = billingClient ?: return Result.failure(IllegalStateException("BillingClient not connected"))
        if (context !is Activity) return Result.failure(IllegalStateException("Context must be Activity"))

        // Store for listener callback
        lastLaunchedSkuId = skuId
        lastLaunchedOrderId = obfuscatedAccountId

        val productParams = QueryProductDetailsParams.Product.newBuilder()
            .setProductId(skuId).setProductType(BillingClient.ProductType.INAPP).build()
        val params = QueryProductDetailsParams.newBuilder().setProductList(listOf(productParams)).build()

        return suspendCancellableCoroutine { cont ->
            client.queryProductDetailsAsync(params) { _, details ->
                val productDetail = details?.firstOrNull()
                if (productDetail == null) {
                    cont.resume(Result.failure(RuntimeException("Product not found: $skuId")))
                    return@queryProductDetailsAsync
                }
                val billingParams = BillingFlowParams.newBuilder()
                    .setProductDetailsParamsList(listOf(
                        BillingFlowParams.ProductDetailsParams.newBuilder()
                            .setProductDetails(productDetail).build()
                    ))
                    .setObfuscatedAccountId(obfuscatedAccountId)
                    .build()
                val responseCode = client.launchBillingFlow(context as Activity, billingParams)
                when (responseCode.responseCode) {
                    BillingClient.BillingResponseCode.OK -> cont.resume(Result.success(Unit))
                    BillingClient.BillingResponseCode.USER_CANCELED -> cont.resume(Result.failure(RuntimeException("User cancelled")))
                    else -> cont.resume(Result.failure(RuntimeException("launchBillingFlow failed: ${responseCode.responseCode}")))
                }
            }
        }
    }

    override suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit> {
        val client = billingClient ?: return Result.failure(IllegalStateException("BillingClient not connected"))
        return suspendCancellableCoroutine { cont ->
            client.acknowledgePurchase(
                AcknowledgePurchaseParams.newBuilder().setPurchaseToken(purchaseToken).build()
            ) { billingResult ->
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK)
                    cont.resume(Result.success(Unit))
                else cont.resume(Result.failure(RuntimeException("acknowledge failed: ${billingResult.responseCode}")))
            }
        }
    }

    override fun disconnect() {
        billingClient?.endConnection()
        billingClient = null
    }
}
