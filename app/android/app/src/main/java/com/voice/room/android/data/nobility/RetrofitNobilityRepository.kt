package com.voice.room.android.data.nobility

import com.voice.room.android.domain.nobility.*
import java.util.UUID

class RetrofitNobilityRepository(
    private val api: NobilityApiService
) : INobilityRepository {

    override suspend fun listTiers(): Result<List<NobleTier>> = runCatching {
        val response = api.listTiers()
        if (!response.isSuccessful) throw RuntimeException("${response.code()}: ${response.message()}")
        response.body()!!.tiers.map { it.toDomain() }
    }

    override suspend fun getMyNoble(): Result<MyNoble?> = runCatching {
        val response = api.getMyNoble()
        if (!response.isSuccessful) throw RuntimeException("${response.code()}: ${response.message()}")
        val body = response.body() ?: return@runCatching null
        if (body.tier_id.isEmpty() || body.level == null) return@runCatching null
        MyNoble(
            tierId = body.tier_id,
            tierName = body.tier_name ?: "",
            level = body.level ?: 0,
            badgeColor = body.badge_color ?: "",
            entranceAnimationUrl = body.entrance_animation_url,
            bgmUrl = body.bgm_url,
            startAt = body.start_at ?: "",
            expireAt = body.expire_at ?: "",
            autoRenew = body.auto_renew ?: false
        )
    }

    override suspend fun purchase(
        tierId: String,
        autoRenew: Boolean
    ): Result<PurchaseNobleResult> = runCatching {
        val response = api.purchase(
            PurchaseRequest(
                tier_id = tierId,
                msg_id = UUID.randomUUID().toString(),
                auto_renew = autoRenew
            )
        )
        if (!response.isSuccessful) throw RuntimeException("${response.code()}: ${response.message()}")
        val body = response.body()!!
        PurchaseNobleResult(body.tier_id, body.new_expire_at, body.diamonds_deducted)
    }

    private fun NobleTierDto.toDomain() = NobleTier(
        tierId = tier_id,
        nameEn = name_en,
        nameAr = name_ar,
        level = level,
        monthlyDiamonds = monthly_diamonds,
        monthlyUsd = monthly_usd,
        privileges = privileges,
        iconUrl = icon_url,
        entranceAnimationUrl = entrance_animation_url,
        bgmUrl = bgm_url,
        badgeColor = badge_color,
        frameUrl = frame_url
    )
}
