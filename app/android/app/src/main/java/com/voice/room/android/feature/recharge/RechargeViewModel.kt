package com.voice.room.android.feature.recharge

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.payment.IPaymentRepository
import com.voice.room.android.domain.payment.SkuItem
import com.voice.room.android.domain.wallet.IWalletRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class RechargeViewModel(
    private val paymentRepo: IPaymentRepository,
    private val walletRepo: IWalletRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(RechargeUiState())
    val uiState: StateFlow<RechargeUiState> = _uiState.asStateFlow()

    init {
        loadSkus()
        loadBalance()
    }

    fun loadSkus() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingSkus = true, error = null) }
            paymentRepo.listSkus()
                .onSuccess { skus ->
                    _uiState.update {
                        it.copy(
                            skus = skus.filter { s -> s.isActive }.sortedBy { s -> s.sortOrder },
                            isLoadingSkus = false
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoadingSkus = false, error = e.message) }
                }
        }
    }

    fun loadBalance() {
        viewModelScope.launch {
            walletRepo.getBalance()
                .onSuccess { balance ->
                    _uiState.update { it.copy(balance = balance) }
                }
        }
    }

    fun selectSku(sku: SkuItem?) {
        _uiState.update { it.copy(selectedSku = sku) }
    }

    fun createOrder() {
        val sku = _uiState.value.selectedSku ?: return
        viewModelScope.launch {
            _uiState.update { it.copy(isCreatingOrder = true, error = null) }
            paymentRepo.createOrder(sku.skuId)
                .onSuccess { result ->
                    _uiState.update {
                        it.copy(
                            isCreatingOrder = false,
                            orderCreated = true,
                            createdOrderId = result.orderId
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isCreatingOrder = false, error = e.message) }
                }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    companion object {
        fun factory(
            paymentRepo: IPaymentRepository,
            walletRepo: IWalletRepository
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return RechargeViewModel(paymentRepo, walletRepo) as T
            }
        }
    }
}
