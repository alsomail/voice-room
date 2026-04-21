package com.voice.room.android.data.wallet

import androidx.paging.PagingConfig
import androidx.paging.PagingSource
import androidx.paging.PagingSource.LoadParams
import androidx.paging.PagingSource.LoadResult
import androidx.paging.PagingState
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.TxnsPage
import com.voice.room.android.domain.wallet.WalletTxn
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — WalletTxnPagingSource (T-30027)
 *
 * PS-01: Refresh(key=null, size=20), total=42 → Page(prevKey=null, nextKey=2, 20 items)
 * PS-02: Append(key=2, size=20), total=42     → Page(prevKey=1, nextKey=3, 20 items)
 * PS-03: 最后一页，items < size              → nextKey=null
 * PS-04: listTxns 抛出 IOException           → LoadResult.Error
 * PS-05: getRefreshKey, anchorPage.prevKey=1  → 返回 2 (prevKey+1)
 * PS-06: getRefreshKey 首页 prevKey=null      → 返回 null
 */
class WalletTxnPagingSourceTest {

    // ─── Fixture Helpers ─────────────────────────────────────────────────────

    private fun makeTxn(id: String) = WalletTxn(
        id = id,
        amount = 100L,
        reason = "Test $id",
        iconUrl = null,
        createdAt = "2026-01-01T00:00:00Z",
    )

    private fun makeItems(count: Int): List<WalletTxn> =
        (1..count).map { makeTxn("t-$it") }

    private fun repoReturning(total: Int, page: Int, items: List<WalletTxn>): IWalletRepository =
        object : IWalletRepository {
            override fun walletPreviewLabel(): String = "Test"
            override suspend fun getBalance(): Result<Long> = Result.success(0L)
            override suspend fun listTxns(p: Int, size: Int): Result<TxnsPage> =
                Result.success(TxnsPage(items = items, total = total, page = page))
        }

    private fun failingRepo(): IWalletRepository =
        object : IWalletRepository {
            override fun walletPreviewLabel(): String = "Test"
            override suspend fun getBalance(): Result<Long> = Result.success(0L)
            override suspend fun listTxns(p: Int, size: Int): Result<TxnsPage> =
                Result.failure(IOException("Network error"))
        }

    // ─── PS-01: Refresh 首页，total=42 → nextKey=2 ───────────────────────────

    @Test
    fun `PS-01 load Refresh key null size 20 total 42 returns 20 items prevKeyNull nextKey2`() =
        runTest {
            val items = makeItems(20)
            val source = WalletTxnPagingSource(repoReturning(total = 42, page = 1, items = items))

            val result = source.load(
                LoadParams.Refresh(key = null, loadSize = 20, placeholdersEnabled = false)
            )

            assertTrue("Should be LoadResult.Page", result is LoadResult.Page)
            val page = result as LoadResult.Page
            assertEquals("data.size should be 20", 20, page.data.size)
            assertNull("prevKey should be null for page 1", page.prevKey)
            assertEquals("nextKey should be 2", 2, page.nextKey)
        }

    // ─── PS-02: Append 第 2 页，total=42 → prevKey=1, nextKey=3 ─────────────

    @Test
    fun `PS-02 load Append key 2 size 20 total 42 returns prevKey1 nextKey3`() = runTest {
        val items = makeItems(20)
        val source = WalletTxnPagingSource(repoReturning(total = 42, page = 2, items = items))

        val result = source.load(
            LoadParams.Append(key = 2, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue(result is LoadResult.Page)
        val page = result as LoadResult.Page
        assertEquals("prevKey should be 1", 1, page.prevKey)
        assertEquals("nextKey should be 3", 3, page.nextKey)
        assertEquals("data.size should be 20", 20, page.data.size)
    }

    // ─── PS-03: 最后一页，items < size → nextKey=null ────────────────────────

    @Test
    fun `PS-03 load last page total 42 page 3 2 items returns prevKey2 nextKeyNull`() = runTest {
        val items = makeItems(2)
        val source = WalletTxnPagingSource(repoReturning(total = 42, page = 3, items = items))

        val result = source.load(
            LoadParams.Append(key = 3, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue(result is LoadResult.Page)
        val page = result as LoadResult.Page
        assertEquals("data.size should be 2", 2, page.data.size)
        assertEquals("prevKey should be 2", 2, page.prevKey)
        assertNull("nextKey should be null (last page)", page.nextKey)
    }

    // ─── PS-04: listTxns 抛 IOException → LoadResult.Error ──────────────────

    @Test
    fun `PS-04 load when repo fails with IOException returns LoadResult Error`() = runTest {
        val source = WalletTxnPagingSource(failingRepo())

        val result = source.load(
            LoadParams.Refresh(key = null, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue("Should be LoadResult.Error", result is LoadResult.Error)
        val error = result as LoadResult.Error
        assertTrue("cause should be IOException", error.throwable is IOException)
        assertEquals("Network error", error.throwable.message)
    }

    // ─── PS-05: getRefreshKey, anchorPage.prevKey=1 → 返回 2 ─────────────────

    @Test
    fun `PS-05 getRefreshKey with prevKey 1 returns 2`() = runTest {
        val source = WalletTxnPagingSource(repoReturning(total = 42, page = 2, items = makeItems(20)))

        val anchorPage = LoadResult.Page(
            data = makeItems(20),
            prevKey = 1,
            nextKey = 3,
        )
        val pagingState = PagingState(
            pages = listOf(anchorPage),
            anchorPosition = 5,
            config = PagingConfig(pageSize = 20),
            leadingPlaceholderCount = 0,
        )

        val refreshKey = source.getRefreshKey(pagingState)
        assertEquals("refreshKey should be prevKey(1)+1 = 2", 2, refreshKey)
    }

    // ─── PS-06: getRefreshKey 首页 prevKey=null → 返回 null ──────────────────

    @Test
    fun `PS-06 getRefreshKey first page prevKey null returns null`() = runTest {
        val source = WalletTxnPagingSource(repoReturning(total = 2, page = 1, items = makeItems(2)))

        val firstPage = LoadResult.Page<Int, WalletTxn>(
            data = makeItems(2),
            prevKey = null,
            nextKey = null,
        )
        val pagingState = PagingState(
            pages = listOf(firstPage),
            anchorPosition = 0,
            config = PagingConfig(pageSize = 20),
            leadingPlaceholderCount = 0,
        )

        assertNull("Should return null when no keys available", source.getRefreshKey(pagingState))
    }

    // ─── PS-07: 空结果 → nextKey=null ────────────────────────────────────────

    @Test
    fun `PS-07 empty items returns nextKey null`() = runTest {
        val source = WalletTxnPagingSource(repoReturning(total = 0, page = 1, items = emptyList()))

        val result = source.load(
            LoadParams.Refresh(key = null, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue(result is LoadResult.Page)
        val page = result as LoadResult.Page
        assertEquals("data should be empty", 0, page.data.size)
        assertNull("prevKey should be null", page.prevKey)
        assertNull("nextKey should be null for empty result", page.nextKey)
    }
}
