package com.voice.room.android.data.payment

import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.Path

/**
 * Payment API Retrofit 接口 (T-30060~63)
 *
 * 协议契约：与 App Server 路由完全一致
 * - GET  /v1/payments/skus
 * - POST /v1/payments/orders
 * - POST /v1/payments/google/verify
 */
interface PaymentApiService {

    @GET("payments/skus")
    suspend fun listSkus(): Response<SkusResponse>

    @POST("payments/orders")
    suspend fun createOrder(@Body body: CreateOrderRequest): Response<CreateOrderResponse>

    @POST("payments/google/verify")
    suspend fun verifyPurchase(@Body body: VerifyRequest): Response<VerifyResponse>
}

// ─── Request bodies ─────────────────────────────────────────────────────────

data class CreateOrderRequest(val sku_id: String)

data class VerifyRequest(
    val order_id: String,
    val purchase_token: String
)

// ─── Response bodies (snake_case matches App Server serde) ─────────────────

data class SkusResponse(val skus: List<SkuDto>)

data class SkuDto(
    val sku_id: String,
    val provider: String,
    val diamonds: Long,
    val display_price_usd: String,
    val display_price_local: String?,
    val display_currency: String?,
    val is_active: Boolean,
    val sort_order: Int,
    val tag: String?
)

data class CreateOrderResponse(
    val order_id: String,
    val sku: SkuDto
)

data class VerifyResponse(
    val order_id: String,
    val new_state: String,
    val diamonds_credited: Long
)
