package com.voice.room.android.feature.recharge

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.WalletTxn
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class RechargeHistoryState(
    val transactions: List<WalletTxn> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null
)

class RechargeHistoryViewModel(
    private val walletRepo: IWalletRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(RechargeHistoryState())
    val uiState: StateFlow<RechargeHistoryState> = _uiState.asStateFlow()

    init { loadHistory() }

    fun loadHistory() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            walletRepo.listTxns(1, 50)
                .onSuccess { page ->
                    val recharges = page.items.filter {
                        it.reason.contains("recharge", ignoreCase = true) || it.amount > 0
                    }
                    _uiState.update { it.copy(transactions = recharges, isLoading = false) }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, error = e.message) }
                }
        }
    }

    companion object {
        fun factory(walletRepo: IWalletRepository): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    RechargeHistoryViewModel(walletRepo) as T
            }
    }
}
