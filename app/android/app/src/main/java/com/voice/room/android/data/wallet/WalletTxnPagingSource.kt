package com.voice.room.android.data.wallet

import androidx.paging.PagingSource
import androidx.paging.PagingState
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.WalletTxn

/**
 * Paging3 PagingSource — 钱包流水分页数据源 (T-30027)
 *
 * - 使用 1-based 页码（Key = Int）
 * - [load] 调用 [IWalletRepository.listTxns] 获取数据
 * - 最后一页判断：`items.size < loadSize` 或 `items.isEmpty()`
 * - [getRefreshKey] 标准实现：`anchorPage.prevKey + 1`
 *
 * @param walletRepository 钱包 Repository，提供 listTxns 实现
 */
class WalletTxnPagingSource(
    private val walletRepository: IWalletRepository,
) : PagingSource<Int, WalletTxn>() {

    override suspend fun load(params: LoadParams<Int>): LoadResult<Int, WalletTxn> {
        val page = params.key ?: 1
        val pageSize = params.loadSize.coerceAtMost(100)

        return walletRepository.listTxns(page = page, size = pageSize)
            .fold(
                onSuccess = { txnsPage ->
                    val items = txnsPage.items
                    val prevKey = if (page == 1) null else page - 1
                    val nextKey = if (items.isEmpty() || items.size < pageSize) null else page + 1
                    LoadResult.Page(
                        data = items,
                        prevKey = prevKey,
                        nextKey = nextKey,
                    )
                },
                onFailure = { throwable ->
                    LoadResult.Error(throwable)
                },
            )
    }

    override fun getRefreshKey(state: PagingState<Int, WalletTxn>): Int? {
        return state.anchorPosition?.let { anchorPosition ->
            val anchorPage = state.closestPageToPosition(anchorPosition)
            anchorPage?.prevKey?.plus(1) ?: anchorPage?.nextKey?.minus(1)
        }
    }
}
