package com.voice.room.android.data.wallet

import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.TxnsPage
import com.voice.room.android.domain.wallet.WalletTxn

/**
 * Debug / 测试占位实现（T-30027）
 *
 * 返回固定 stub 数据，供开发期与 AppContainerTest 使用。
 */
class DebugWalletRepository : IWalletRepository {

    override fun walletPreviewLabel(): String = "Wallet module reserved"

    override suspend fun getBalance(): Result<Long> = Result.success(888L)

    override suspend fun listTxns(page: Int, size: Int): Result<TxnsPage> = Result.success(
        TxnsPage(
            items = listOf(
                WalletTxn(id = "debug-1", amount = 100L, reason = "Debug 收入", createdAt = "2026-01-01T00:00:00Z"),
                WalletTxn(id = "debug-2", amount = -50L, reason = "Debug 支出", createdAt = "2026-01-02T00:00:00Z"),
            ),
            total = 2,
            page = page,
        )
    )
}
