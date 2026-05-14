package com.voice.room.android.data.nobility

import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST

/**
 * Nobility API Retrofit 接口 (T-30070~71)
 *
 * - GET  /v1/nobles/tiers
 * - GET  /v1/nobles/me
 * - POST /v1/nobles/purchase
 */
interface NobilityApiService {

    @GET("nobles/tiers")
    suspend fun listTiers(): Response<TiersResponse>

    @GET("nobles/me")
    suspend fun getMyNoble(): Response<MyNobleResponse>

    @POST("nobles/purchase")
    suspend fun purchase(@Body body: PurchaseRequest): Response<PurchaseResponse>
}

// ─── Request ────────────────────────────────────────────────────────────────

data class PurchaseRequest(
    val tier_id: String,
    val msg_id: String,
    val auto_renew: Boolean
)

// ─── Responses ──────────────────────────────────────────────────────────────

data class TiersResponse(val tiers: List<NobleTierDto>)

data class NobleTierDto(
    val tier_id: String,
    val name_en: String,
    val name_ar: String,
    val level: Int,
    val monthly_diamonds: Long,
    val monthly_usd: String,
    val privileges: Map<String, Any>?,
    val icon_url: String,
    val entrance_animation_url: String?,
    val bgm_url: String?,
    val badge_color: String,
    val frame_url: String?
)

data class MyNobleResponse(
    val tier_id: String,
    val tier_name: String?,
    val level: Int?,
    val badge_color: String?,
    val entrance_animation_url: String?,
    val bgm_url: String?,
    val start_at: String?,
    val expire_at: String?,
    val auto_renew: Boolean?
)

data class PurchaseResponse(
    val tier_id: String,
    val new_expire_at: String,
    val diamonds_deducted: Long?
)
