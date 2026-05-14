package com.voice.room.android.feature.noble

import com.voice.room.android.domain.nobility.MyNoble
import com.voice.room.android.domain.nobility.NobleTier

data class NobleCenterUiState(
    val currentNoble: MyNoble? = null,
    val tiers: List<NobleTier> = emptyList(),
    val selectedTierIndex: Int = 0,
    val isLoadingTiers: Boolean = false,
    val isLoadingPurchase: Boolean = false,
    val error: String? = null,
    val purchaseSuccess: Boolean = false
)
