package com.voice.room.android.data.wallet

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.WalletApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.PageDto
import com.voice.room.android.data.remote.model.TxnDto
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.TxnsPage
import com.voice.room.android.domain.wallet.WalletTxn

/**
 * [IWalletRepository] 的 Retrofit 真实实现 (T-30027)
 *
 * - [getBalance]   → GET /api/v1/wallet/balance，成功返回 Long 余额
 * - [listTxns]     → GET /api/v1/wallet/transactions?page=&size=，成功返回 [TxnsPage]
 * - Authorization header 由 AuthInterceptor 自动注入
 * - HTTP 4xx/5xx 统一解析为 [ApiException]
 */
class RetrofitWalletRepository(
    private val apiService: WalletApiService,
) : IWalletRepository {

    private val gson = Gson()

    override fun walletPreviewLabel(): String = "Wallet (Retrofit)"

    // ─── getBalance ──────────────────────────────────────────────────────────

    override suspend fun getBalance(): Result<Long> = runCatching {
        val response = apiService.getBalance()
        val data = parseBody(response)
        data.balance
    }

    // ─── listTxns ────────────────────────────────────────────────────────────

    override suspend fun listTxns(page: Int, size: Int): Result<TxnsPage> = runCatching {
        val response = apiService.listTxns(page = page, size = size)
        val pageDto = parseBody(response)
        pageDto.toDomain(page)
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /**
     * 统一 HTTP 响应解析（与 RetrofitUserRepository 保持相同错误处理策略）：
     * - 2xx + code==0 + data≠null → 返回 data
     * - 2xx + code≠0              → 抛出 [ApiException]
     * - 4xx/5xx                   → 解析 error body，抛出 [ApiException]
     */
    private fun <T> parseBody(response: retrofit2.Response<ApiResponse<T>>): T {
        if (response.isSuccessful) {
            val apiBody = response.body()
                ?: throw ApiException(-1, "Empty response body")
            if (apiBody.code == 0 && apiBody.data != null) {
                return apiBody.data
            }
            throw ApiException(apiBody.code, apiBody.message)
        }

        val errorJson = response.errorBody()?.string()
        if (!errorJson.isNullOrBlank()) {
            runCatching {
                val type = object : TypeToken<ApiResponse<Nothing>>() {}.type
                val errorBody: ApiResponse<Nothing> = gson.fromJson(errorJson, type)
                throw ApiException(errorBody.code, errorBody.message)
            }.onSuccess { /* unreachable */ }
                .onFailure { if (it is ApiException) throw it }
        }
        throw ApiException(response.code(), "HTTP ${response.code()}: ${response.message()}")
    }

    // ─── DTO → Domain ─────────────────────────────────────────────────────────

    private fun PageDto<TxnDto>.toDomain(page: Int): TxnsPage = TxnsPage(
        items = items.map { it.toDomain() },
        total = total,
        page = page,
    )

    private fun TxnDto.toDomain(): WalletTxn = WalletTxn(
        id = id,
        amount = amount,
        reason = reason,
        iconUrl = iconUrl,
        createdAt = createdAt,
    )
}
