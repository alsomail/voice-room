package com.voice.room.android.domain.wallet

/**
 * 钱包 Repository 契约接口 (T-30027)
 *
 * - [walletPreviewLabel]   调试用标签（向后兼容 AppContainerTest）
 * - [getBalance]           获取当前钻石余额
 * - [listTxns]             分页获取流水列表（供 WalletTxnPagingSource 调用）
 */
interface IWalletRepository {
    /** 调试用标签，AppContainerTest 断言非空 */
    fun walletPreviewLabel(): String

    /** 获取当前钻石余额。成功返回 [Result.success(Long)]；失败返回 [Result.failure] */
    suspend fun getBalance(): Result<Long>

    /**
     * 分页获取流水列表。
     *
     * @param page 页码（1-based）
     * @param size 每页条数，默认 20
     */
    suspend fun listTxns(page: Int, size: Int = 20): Result<TxnsPage>
}
