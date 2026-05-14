package com.voice.room.android.feature.recharge

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.domain.payment.IBillingPort
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
    private val walletRepo: IWalletRepository,
    private val billingPort: IBillingPort?,
    private val pendingHandler: PendingPurchaseHandler?
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

    /**
     * T-30062: 创建订单 → 唤起 Billing → verify + ack (T-30063)
     */
    fun createOrderAndPay() {
        val sku = _uiState.value.selectedSku ?: return
        viewModelScope.launch {
            _uiState.update { it.copy(isCreatingOrder = true, error = null) }

            // Step 1: Create order
            val orderResult = paymentRepo.createOrder(sku.skuId)
            if (orderResult.isFailure) {
                _uiState.update { it.copy(isCreatingOrder = false, error = orderResult.exceptionOrNull()?.message) }
                return@launch
            }
            val order = orderResult.getOrThrow()

            // Step 2: Launch Billing via BillingPort (or dev mock path)
            val billing = billingPort
            if (billing != null) {
                // Connect and launch
                billing.connect()
                    .onFailure {
                        _uiState.update { it.copy(isCreatingOrder = false, error = "Billing service unavailable") }
                        return@launch
                    }

                val tokenResult = billing.launchBillingFlow(sku.skuId, order.orderId)
                if (tokenResult.isFailure) {
                    _uiState.update { it.copy(isCreatingOrder = false, error = tokenResult.exceptionOrNull()?.message) }
                    return@launch
                }

                val token = tokenResult.getOrNull()
                if (token == null) {
                    // User cancelled
                    _uiState.update { it.copy(isCreatingOrder = false) }
                    return@launch
                }

                // Step 3: Save pending + verify
                pendingHandler?.savePending(order.orderId, token)

                // Step 4: Verify
                val verifyResult = paymentRepo.verifyPurchase(order.orderId, token)
                if (verifyResult.isSuccess) {
                    // Step 5: Acknowledge
                    billing.acknowledgePurchase(token)
                    pendingHandler?.removePending(order.orderId)
                    _uiState.update {
                        it.copy(isCreatingOrder = false, orderCreated = true, createdOrderId = order.orderId)
                    }
                    loadBalance()
                    billing.disconnect()
                } else {
                    _uiState.update {
                        it.copy(isCreatingOrder = false, error = verifyResult.exceptionOrNull()?.message)
                    }
                }
            } else {
                // No BillingPort (test/dev): just mark order created
                _uiState.update {
                    it.copy(isCreatingOrder = false, orderCreated = true, createdOrderId = order.orderId)
                }
            }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    companion object {
        fun factory(
            paymentRepo: IPaymentRepository,
            walletRepo: IWalletRepository,
            billingPort: IBillingPort? = null,
            pendingHandler: PendingPurchaseHandler? = null
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return RechargeViewModel(paymentRepo, walletRepo, billingPort, pendingHandler) as T
            }
        }
    }
}
