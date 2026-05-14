package com.voice.room.android.data.payment

import com.voice.room.android.domain.payment.*

class RetrofitPaymentRepository(
    private val api: PaymentApiService
) : IPaymentRepository {

    override suspend fun listSkus(): Result<List<SkuItem>> = runCatching {
        val response = api.listSkus()
        if (!response.isSuccessful) throw ApiException(response.code(), response.message())
        response.body()!!.skus.map { it.toDomain() }
    }

    override suspend fun createOrder(skuId: String): Result<CreateOrderResult> = runCatching {
        val response = api.createOrder(CreateOrderRequest(sku_id = skuId))
        if (!response.isSuccessful) throw ApiException(response.code(), response.message())
        val body = response.body()!!
        CreateOrderResult(
            orderId = body.order_id,
            skuId = body.sku.sku_id,
            diamonds = body.sku.diamonds,
            displayPriceUsd = body.sku.display_price_usd
        )
    }

    override suspend fun verifyPurchase(
        orderId: String,
        purchaseToken: String
    ): Result<VerifyResult> = runCatching {
        val response = api.verifyPurchase(
            VerifyRequest(order_id = orderId, purchase_token = purchaseToken)
        )
        if (!response.isSuccessful) throw ApiException(response.code(), response.message())
        val body = response.body()!!
        VerifyResult(body.order_id, body.state, body.diamonds_credited ?: 0L)
    }

    private fun SkuDto.toDomain() = SkuItem(
        skuId = sku_id,
        provider = provider,
        diamonds = diamonds,
        displayPriceUsd = display_price_usd,
        displayPriceLocal = display_price_local,
        displayCurrency = display_currency,
        isActive = is_active,
        sortOrder = sort_order,
        tag = tag
    )
}

class ApiException(code: Int, message: String) : Exception("$code: $message")
