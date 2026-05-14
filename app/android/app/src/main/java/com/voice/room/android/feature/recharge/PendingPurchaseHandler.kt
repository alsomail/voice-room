package com.voice.room.android.feature.recharge

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.domain.payment.IPaymentRepository
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.pendingPurchaseStore by preferencesDataStore("pending_purchases")

/**
 * PendingPurchaseHandler — 待处理购买容错机制 (T-30063)
 *
 * 在 verify 成功前将 purchase 持久化到 DataStore；
 * App 冷启动时自动重试未完成的 token。
 */
class PendingPurchaseHandler(
    private val context: Context,
    private val paymentRepo: IPaymentRepository
) {
    private val store get() = context.pendingPurchaseStore
    private val gson = Gson()

    private val pendingKey = stringPreferencesKey("pending_purchases")

    suspend fun savePending(orderId: String, purchaseToken: String) {
        val current = loadPending().toMutableList()
        current.add(PendingPurchase(orderId, purchaseToken, System.currentTimeMillis()))
        store.edit { it[pendingKey] = gson.toJson(current) }
    }

    suspend fun removePending(orderId: String) {
        val current = loadPending().toMutableList()
        current.removeAll { it.orderId == orderId }
        store.edit { it[pendingKey] = gson.toJson(current) }
    }

    suspend fun loadPending(): List<PendingPurchase> {
        val raw = store.data.map { it[pendingKey] }.first() ?: return emptyList()
        return try {
            val type = object : TypeToken<List<PendingPurchase>>() {}.type
            gson.fromJson<List<PendingPurchase>>(raw, type) ?: emptyList()
        } catch (_: Exception) { emptyList() }
    }

    /** Retry all pending purchases on app cold start (T-30063 #2) */
    suspend fun retryPending(maxRetries: Int = 3) {
        val pending = loadPending()
        for (pp in pending) {
            if (pp.retries >= maxRetries) {
                removePending(pp.orderId)
                continue
            }
            val result = paymentRepo.verifyPurchase(pp.orderId, pp.purchaseToken)
            if (result.isSuccess) {
                removePending(pp.orderId)
            } else {
                // Update retry count
                val updated = pending.map {
                    if (it.orderId == pp.orderId) it.copy(retries = it.retries + 1) else it
                }
                store.edit { it[pendingKey] = gson.toJson(updated) }
            }
        }
    }
}

data class PendingPurchase(
    val orderId: String,
    val purchaseToken: String,
    val savedAt: Long,
    val retries: Int = 0
)
