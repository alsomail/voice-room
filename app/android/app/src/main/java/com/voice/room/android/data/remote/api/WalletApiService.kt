package com.voice.room.android.data.remote.api

import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.BalanceDto
import com.voice.room.android.data.remote.model.PageDto
import com.voice.room.android.data.remote.model.TxnDto
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.Query

/**
 * 钱包 Retrofit API 接口 (T-30027)
 *
 * Base URL：`/api/v1/`
 * Authorization header 由 AuthInterceptor 自动注入
 */
interface WalletApiService {

    /**
     * 获取当前用户钻石余额
     *
     * GET /api/v1/wallet/balance
     */
    @GET("wallet/balance")
    suspend fun getBalance(): Response<ApiResponse<BalanceDto>>

    /**
     * 分页获取流水列表
     *
     * GET /api/v1/wallet/transactions?page={p}&size={s}
     *
     * @param page 页码（1-based）
     * @param size 每页条数，默认 20
     * @param type 可选流水类型过滤（null = 全部）
     */
    @GET("wallet/transactions")
    suspend fun listTxns(
        @Query("page") page: Int,
        @Query("size") size: Int = 20,
        @Query("type") type: String? = null,
    ): Response<ApiResponse<PageDto<TxnDto>>>
}
