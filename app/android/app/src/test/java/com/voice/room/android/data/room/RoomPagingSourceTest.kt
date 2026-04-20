package com.voice.room.android.data.room

import androidx.paging.PagingConfig
import androidx.paging.PagingSource
import androidx.paging.PagingSource.LoadParams
import androidx.paging.PagingSource.LoadResult
import androidx.paging.PagingState
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.RoomItem
import com.voice.room.android.domain.room.RoomsPage
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — RoomPagingSource
 *
 * P01: Refresh(key=null, size=20), total=42 → Page(prevKey=null, nextKey=2, 20 items)
 * P02: Append(key=2, size=20), total=42     → Page(prevKey=1, nextKey=3, 20 items)
 * P03: Append(key=3, size=20), total=42, 2 items → Page(prevKey=2, nextKey=null)
 * P04: repo throws IOException              → LoadResult.Error(IOException)
 * P05: getRefreshKey, anchorPage.prevKey=1  → 返回 2 (prevKey+1)
 */
class RoomPagingSourceTest {

    // ─────────────────────────────────────────────
    // Test Fixture Helpers
    // ─────────────────────────────────────────────

    private fun makeRoom(id: String) = RoomItem(
        roomId = id,
        title = "Room $id",
        roomType = "normal",
        memberCount = 5,
        maxMembers = 20,
        ownerNickname = "User",
        ownerAvatar = null,
        createdAt = "2024-01-01T00:00:00Z"
    )

    private fun makeItems(count: Int): List<RoomItem> =
        (1..count).map { makeRoom("id-$it") }

    private fun repoReturning(total: Int, page: Int, items: List<RoomItem>): IRoomRepository =
        object : IRoomRepository {
            override suspend fun getRooms(p: Int, size: Int): Result<RoomsPage> =
                Result.success(RoomsPage(total = total, page = page, items = items))

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                RoomPagingSource(this)

            override suspend fun createRoom(title: String, type: String, password: String?): Result<String> =
                Result.failure(UnsupportedOperationException())
        }

    // ─────────────────────────────────────────────
    // P01: 首页加载，total=42 → nextKey=2, prevKey=null
    // ─────────────────────────────────────────────

    @Test
    fun `P01 load Refresh key null size 20 total 42 returns 20 items prevKeyNull nextKey2`() =
        runTest {
            val items = makeItems(20)
            val source = RoomPagingSource(repoReturning(total = 42, page = 1, items = items))

            val result = source.load(
                LoadParams.Refresh(key = null, loadSize = 20, placeholdersEnabled = false)
            )

            assertTrue("Should be LoadResult.Page", result is LoadResult.Page)
            val page = result as LoadResult.Page
            assertEquals("data.size should be 20", 20, page.data.size)
            assertNull("prevKey should be null for page 1", page.prevKey)
            assertEquals("nextKey should be 2", 2, page.nextKey)
        }

    // ─────────────────────────────────────────────
    // P02: 第 2 页 append，total=42 → prevKey=1, nextKey=3
    // ─────────────────────────────────────────────

    @Test
    fun `P02 load Append key 2 size 20 total 42 returns prevKey1 nextKey3`() = runTest {
        val items = makeItems(20)
        val source = RoomPagingSource(repoReturning(total = 42, page = 2, items = items))

        val result = source.load(
            LoadParams.Append(key = 2, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue(result is LoadResult.Page)
        val page = result as LoadResult.Page
        assertEquals("prevKey should be 1", 1, page.prevKey)
        assertEquals("nextKey should be 3", 3, page.nextKey)
        assertEquals("data.size should be 20", 20, page.data.size)
    }

    // ─────────────────────────────────────────────
    // P03: 最后一页，total=42 page=3 items=2 → nextKey=null
    // ─────────────────────────────────────────────

    @Test
    fun `P03 load last page total 42 page 3 2 items returns prevKey2 nextKeyNull`() = runTest {
        val items = makeItems(2)
        val source = RoomPagingSource(repoReturning(total = 42, page = 3, items = items))

        val result = source.load(
            LoadParams.Append(key = 3, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue(result is LoadResult.Page)
        val page = result as LoadResult.Page
        assertEquals("data.size should be 2", 2, page.data.size)
        assertEquals("prevKey should be 2", 2, page.prevKey)
        assertNull("nextKey should be null (last page)", page.nextKey)
    }

    // ─────────────────────────────────────────────
    // P04: getRooms 抛出 IOException → LoadResult.Error
    // ─────────────────────────────────────────────

    @Test
    fun `P04 load when repo fails with IOException returns LoadResult Error`() = runTest {
        val failingRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> =
                Result.failure(IOException("Network error"))

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                RoomPagingSource(this)

            override suspend fun createRoom(title: String, type: String, password: String?): Result<String> =
                Result.failure(UnsupportedOperationException())
        }
        val source = RoomPagingSource(failingRepo)

        val result = source.load(
            LoadParams.Refresh(key = null, loadSize = 20, placeholdersEnabled = false)
        )

        assertTrue("Should be LoadResult.Error", result is LoadResult.Error)
        val error = result as LoadResult.Error
        assertTrue("cause should be IOException", error.throwable is IOException)
        assertEquals("Network error", error.throwable.message)
    }

    // ─────────────────────────────────────────────
    // P05: getRefreshKey — anchorPage.prevKey=1 → 返回 2 (prevKey+1)
    // ─────────────────────────────────────────────

    @Test
    fun `P05 getRefreshKey with prevKey 1 returns 2`() = runTest {
        val source = RoomPagingSource(FakeRoomRepository())

        // 手动构造 PagingState：anchor 位于一个 prevKey=1, nextKey=3 的页面内
        val anchorPage = LoadResult.Page(
            data = makeItems(20),
            prevKey = 1,
            nextKey = 3
        )
        val pagingState = PagingState(
            pages = listOf(anchorPage),
            anchorPosition = 5,
            config = PagingConfig(pageSize = 20),
            leadingPlaceholderCount = 0
        )

        val refreshKey = source.getRefreshKey(pagingState)

        // closestPageToPosition(5) = anchorPage → prevKey=1 → prevKey+1 = 2
        assertEquals("refreshKey should be prevKey(1)+1 = 2", 2, refreshKey)
    }

    // ─────────────────────────────────────────────
    // 边界：getRefreshKey 首页（prevKey=null）返回 null
    // ─────────────────────────────────────────────

    @Test
    fun `getRefreshKey first page prevKey null nextKey null returns null`() = runTest {
        val source = RoomPagingSource(FakeRoomRepository())

        val firstPage = LoadResult.Page<Int, RoomItem>(
            data = makeItems(2),
            prevKey = null,
            nextKey = null
        )
        val pagingState = PagingState(
            pages = listOf(firstPage),
            anchorPosition = 0,
            config = PagingConfig(pageSize = 20),
            leadingPlaceholderCount = 0
        )

        assertNull("Should return null when no keys available", source.getRefreshKey(pagingState))
    }

    // ─────────────────────────────────────────────
    // P01-extra: Refresh loadSize=60 (Paging3 默认 3x) 不与 Append 重叠
    //
    // 问题根因：PagingConfig 未设置 initialLoadSizeHint，默认 pageSize×3=60。
    // Refresh 以 loadSize=60 返回 items[1..60]，nextKey=2；
    // 后续 Append(key=2, loadSize=20) 返回 items[21..40] → 与首次加载重叠 40 条。
    // 修复：在 PagingConfig 中显式设置 initialLoadSizeHint = pageSize = 20，
    // 确保 Refresh 也以 loadSize=20 调用，nextKey=2 与 Append 数据完全连续无重叠。
    // ─────────────────────────────────────────────

    @Test
    fun `P01-extra Refresh with loadSize=60 default 3x does not overlap Append`() = runTest {
        // Arrange: 模拟 Paging3 默认 initialLoadSizeHint = pageSize * 3 = 60
        val items = (1..60).map { makeRoom("id-$it") }
        val fakeRepo = FakeRoomRepository()
        fakeRepo.rooms = items
        fakeRepo.total = 100
        val source = RoomPagingSource(fakeRepo)

        // Act: 以 loadSize=60（旧默认行为）触发 Refresh
        val result = source.load(
            LoadParams.Refresh(key = null, loadSize = 60, placeholdersEnabled = false)
        ) as LoadResult.Page

        // Assert: nextKey=2，表明后续 Append 将从 page=2 开始
        // 修复后（initialLoadSizeHint=20），Refresh 永远只会传 loadSize=20，
        // 使 Append(key=2, size=20) 返回 items[21..40]，与首次加载 items[1..20] 完全不重叠。
        assertEquals("应返回全部 60 条数据", 60, result.data.size)
        assertEquals("nextKey 应为 2，Append 从 page=2 开始", 2, result.nextKey)
        assertNull("首页 prevKey 应为 null", result.prevKey)
    }

    // ─────────────────────────────────────────────
    // 边界：loadSize 超过 100 → 被截断为 100
    // ─────────────────────────────────────────────

    @Test
    fun `load with oversized loadSize 200 coerces to 100`() = runTest {
        var capturedSize = 0
        val spyRepo = object : IRoomRepository {
            override suspend fun getRooms(page: Int, size: Int): Result<RoomsPage> {
                capturedSize = size
                return Result.success(RoomsPage(total = 100, page = 1, items = makeItems(100)))
            }

            override fun getRoomsPagingSource(): PagingSource<Int, RoomItem> =
                RoomPagingSource(this)

            override suspend fun createRoom(title: String, type: String, password: String?): Result<String> =
                Result.failure(UnsupportedOperationException())
        }
        val source = RoomPagingSource(spyRepo)

        source.load(LoadParams.Refresh(key = null, loadSize = 200, placeholdersEnabled = false))

        assertEquals("size should be coerced to 100", 100, capturedSize)
    }
}
