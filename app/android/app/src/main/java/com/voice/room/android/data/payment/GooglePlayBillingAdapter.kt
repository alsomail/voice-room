package com.voice.room.android.data.payment

import android.app.Activity
import android.content.Context
import com.android.billingclient.api.*
import com.voice.room.android.domain.payment.IBillingPort
import com.voice.room.android.domain.payment.ProductDetail
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume

/**
 * Google Play BillingClient v6+ 防腐层实现 (T-30061)
 *
 * 封装 BillingClient，业务层通过 [IBillingPort] 调用，
 * 不直接 import com.android.billingclient。
 */
class GooglePlayBillingAdapter(
    private val context: Context
) : IBillingPort {

    private var billingClient: BillingClient? = null
    private var pendingPurchaseResult: BillingResult? = null
    private var pendingPurchases: List<Purchase> = emptyList()

    override suspend fun connect(): Result<Unit> {
        val client = BillingClient.newBuilder(context)
            .setListener { billingResult, purchases ->
                pendingPurchaseResult = billingResult
                if (purchases != null) pendingPurchases = purchases
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

                override fun onBillingServiceDisconnected() {
                    // Will attempt reconnect on next connect() call
                }
            })
        }
    }

    override suspend fun queryProductDetails(
        skuIds: List<String>
    ): Result<List<ProductDetail>> {
        val client = billingClient
            ?: return Result.failure(IllegalStateException("BillingClient not connected"))

        val productList = skuIds.map { id ->
            QueryProductDetailsParams.Product.newBuilder()
                .setProductId(id)
                .setProductType(BillingClient.ProductType.INAPP)
                .build()
        }

        val params = QueryProductDetailsParams.newBuilder()
            .setProductList(productList)
            .build()

        return suspendCancellableCoroutine { cont ->
            client.queryProductDetailsAsync(params) { billingResult, productDetailsList ->
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                    val result = productDetailsList.orEmpty().map { pd ->
                        val price = pd.oneTimePurchaseOfferDetails
                        ProductDetail(
                            productId = pd.productId,
                            price = price?.formattedPrice ?: "",
                            priceCurrencyCode = price?.priceCurrencyCode ?: "",
                            priceAmountMicros = price?.priceAmountMicros ?: 0L,
                            title = pd.title,
                            description = pd.description
                        )
                    }
                    cont.resume(Result.success(result))
                } else {
                    cont.resume(Result.failure(
                        RuntimeException("queryProductDetails failed: ${billingResult.responseCode}")
                    ))
                }
            }
        }
    }

    override suspend fun launchBillingFlow(
        skuId: String,
        obfuscatedAccountId: String
    ): Result<String?> {
        val client = billingClient
            ?: return Result.failure(IllegalStateException("BillingClient not connected"))

        if (context !is Activity) {
            return Result.failure(IllegalStateException("Context must be Activity"))
        }

        val productDetailsParams = QueryProductDetailsParams.Product.newBuilder()
            .setProductId(skuId)
            .setProductType(BillingClient.ProductType.INAPP)
            .build()

        val params = QueryProductDetailsParams.newBuilder()
            .setProductList(listOf(productDetailsParams))
            .build()

        // Query details first, then launch
        return suspendCancellableCoroutine { cont ->
            client.queryProductDetailsAsync(params) { _, details ->
                val productDetail = details?.firstOrNull()
                if (productDetail == null) {
                    cont.resume(Result.failure(RuntimeException("Product not found: $skuId")))
                    return@queryProductDetailsAsync
                }

                val billingParams = BillingFlowParams.newBuilder()
                    .setProductDetailsParamsList(
                        listOf(
                            BillingFlowParams.ProductDetailsParams.newBuilder()
                                .setProductDetails(productDetail)
                                .build()
                        )
                    )
                    .setObfuscatedAccountId(obfuscatedAccountId)
                    .build()

                pendingPurchaseResult = null
                val responseCode = client.launchBillingFlow(context as Activity, billingParams)

                if (responseCode.responseCode == BillingClient.BillingResponseCode.OK) {
                    // Purchase will be delivered via setListener callback
                    // For now, return success — caller will get token from listener
                    cont.resume(Result.success(null)) // token comes from PurchasesUpdatedListener
                } else if (responseCode.responseCode == BillingClient.BillingResponseCode.USER_CANCELED) {
                    cont.resume(Result.success(null))
                } else {
                    cont.resume(Result.failure(
                        RuntimeException("launchBillingFlow failed: ${responseCode.responseCode}")
                    ))
                }
            }
        }
    }

    override suspend fun acknowledgePurchase(purchaseToken: String): Result<Unit> {
        val client = billingClient
            ?: return Result.failure(IllegalStateException("BillingClient not connected"))

        return suspendCancellableCoroutine { cont ->
            val params = AcknowledgePurchaseParams.newBuilder()
                .setPurchaseToken(purchaseToken)
                .build()
            client.acknowledgePurchase(params) { billingResult ->
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                    cont.resume(Result.success(Unit))
                } else {
                    cont.resume(Result.failure(
                        RuntimeException("acknowledge failed: ${billingResult.responseCode}")
                    ))
                }
            }
        }
    }

    override fun disconnect() {
        billingClient?.endConnection()
        billingClient = null
    }
}
