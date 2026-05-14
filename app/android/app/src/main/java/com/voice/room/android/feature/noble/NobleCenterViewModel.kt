package com.voice.room.android.feature.noble

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.nobility.INobilityRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class NobleCenterViewModel(
    private val nobilityRepo: INobilityRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(NobleCenterUiState())
    val uiState: StateFlow<NobleCenterUiState> = _uiState.asStateFlow()

    init { loadTiers() }

    fun loadTiers() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingTiers = true, error = null) }
            nobilityRepo.listTiers()
                .onSuccess { tiers ->
                    // Find current noble's tier index
                    val current = _uiState.value.currentNoble
                    val idx = if (current != null) {
                        tiers.indexOfFirst { it.tierId == current.tierId }.coerceAtLeast(0)
                    } else 0
                    _uiState.update {
                        it.copy(tiers = tiers, isLoadingTiers = false, selectedTierIndex = idx)
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoadingTiers = false, error = e.message) }
                }
        }
    }

    fun loadMyNoble() {
        viewModelScope.launch {
            nobilityRepo.getMyNoble()
                .onSuccess { noble ->
                    _uiState.update { it.copy(currentNoble = noble) }
                }
        }
    }

    fun selectTier(index: Int) {
        _uiState.update { it.copy(selectedTierIndex = index) }
    }

    fun purchase(autoRenew: Boolean) {
        val tier = _uiState.value.tiers.getOrNull(_uiState.value.selectedTierIndex) ?: return
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingPurchase = true, error = null) }
            nobilityRepo.purchase(tier.tierId, autoRenew)
                .onSuccess {
                    _uiState.update {
                        it.copy(isLoadingPurchase = false, purchaseSuccess = true)
                    }
                    loadMyNoble()
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoadingPurchase = false, error = e.message) }
                }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    companion object {
        fun factory(repo: INobilityRepository): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    NobleCenterViewModel(repo) as T
            }
    }
}
