package com.voice.room.android.domain.nobility

/**
 * 贵族 Repository 契约接口 (T-30070~71)
 */
interface INobilityRepository {
    /** GET /v1/nobles/tiers — 获取所有贵族等级 */
    suspend fun listTiers(): Result<List<NobleTier>>

    /** GET /v1/nobles/me — 获取当前用户贵族信息 */
    suspend fun getMyNoble(): Result<MyNoble?>

    /** POST /v1/nobles/purchase — 购买/续费贵族 */
    suspend fun purchase(tierId: String, autoRenew: Boolean): Result<PurchaseNobleResult>
}

data class NobleTier(
    val tierId: String,
    val nameEn: String,
    val nameAr: String,
    val level: Int,
    val monthlyDiamonds: Long,
    val monthlyUsd: String,
    val privileges: Map<String, Any>?,
    val iconUrl: String,
    val entranceAnimationUrl: String?,
    val bgmUrl: String?,
    val badgeColor: String,
    val frameUrl: String?
)

data class MyNoble(
    val tierId: String,
    val tierName: String,
    val level: Int,
    val badgeColor: String,
    val entranceAnimationUrl: String?,
    val bgmUrl: String?,
    val startAt: String,
    val expireAt: String,
    val autoRenew: Boolean
)

data class PurchaseNobleResult(
    val tierId: String,
    val newExpireAt: String,
    val diamondsDeducted: Long?
)
