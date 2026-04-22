package com.voice.room.android.data.ranking

import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.RankingApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.RankingDto
import com.voice.room.android.data.remote.model.RankEntryDto
import com.voice.room.android.data.remote.model.MyRankDto
import okhttp3.MediaType.Companion.toMediaType
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Response

/**
 * TDD 单元测试 — RetrofitRankingRepository (T-30033)
 *
 * REPO-01: 成功响应映射到领域对象
 * REPO-02: code≠0 抛出 ApiException
 * REPO-03: me.rank=null 映射到 MyRank.rank=null（未上榜）
 * REPO-04: HTTP 401 抛出 ApiException(401)
 */
class RetrofitRankingRepositoryTest {

    // ─── Fake API Service ─────────────────────────────────────────────────────

    private class FakeRankingApiService(
        private val response: Response<ApiResponse<RankingDto>>
    ) : RankingApiService {
        override suspend fun getRanking(type: String, period: String, limit: Int): Response<ApiResponse<RankingDto>> =
            response
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun successResponse(dto: RankingDto): Response<ApiResponse<RankingDto>> =
        Response.success(ApiResponse(code = 0, message = "ok", data = dto, requestId = "rid"))

    private fun errorApiResponse(code: Int): Response<ApiResponse<RankingDto>> {
        val body = """{"code":$code,"message":"error","request_id":"rid"}"""
            .toByteArray()
            .let { okhttp3.ResponseBody.create("application/json".toMediaType(), it) }
        return Response.error(code, body)
    }

    private fun buildRepo(service: RankingApiService): RetrofitRankingRepository =
        RetrofitRankingRepository(service)

    // ─── REPO-01: 成功响应映射到领域对象 ─────────────────────────────────────

    @Test
    fun `REPO-01 success response maps to domain RankingPage`() = runTest {
        val dto = RankingDto(
            type = "charm", period = "day", periodKey = "2026-04-22",
            items = listOf(
                RankEntryDto(rank = 1, userId = "u1", nickname = "Alice", avatar = "url1", score = 10000, medal = "gold"),
                RankEntryDto(rank = 2, userId = "u2", nickname = "Bob",   avatar = "url2", score = 8000,  medal = "silver"),
            ),
            me = MyRankDto(rank = 5, score = 3000)
        )
        val repo = buildRepo(FakeRankingApiService(successResponse(dto)))

        val result = repo.getRanking("charm", "day")

        assertTrue(result.isSuccess)
        val page = result.getOrThrow()
        assertEquals("charm", page.type)
        assertEquals("day", page.period)
        assertEquals(2, page.items.size)
        assertEquals(1, page.items[0].rank)
        assertEquals("Alice", page.items[0].nickname)
        assertEquals("gold", page.items[0].medal)
        assertEquals(5, page.me?.rank)
        assertEquals(3000L, page.me?.score)
    }

    // ─── REPO-02: code≠0 → ApiException ──────────────────────────────────────

    @Test
    fun `REPO-02 code not zero throws ApiException`() = runTest {
        val response = Response.success(
            ApiResponse<RankingDto>(code = 40003, message = "invalid param", data = null, requestId = "rid")
        )
        val repo = buildRepo(FakeRankingApiService(response))

        val result = repo.getRanking("charm", "day")

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertNotNull(ex)
        assertTrue(ex is ApiException)
        assertEquals(40003, (ex as ApiException).code)
    }

    // ─── REPO-03: me.rank=null 映射到 MyRank.rank=null ───────────────────────

    @Test
    fun `REPO-03 me rank null maps to MyRank rank null`() = runTest {
        val dto = RankingDto(
            type = "charm", period = "day", periodKey = "2026-04-22",
            items = emptyList(),
            me = MyRankDto(rank = null, score = 0)
        )
        val repo = buildRepo(FakeRankingApiService(successResponse(dto)))

        val result = repo.getRanking("charm", "day")
        val page = result.getOrThrow()

        assertNull("me.rank should be null when not ranked", page.me?.rank)
        assertEquals(0L, page.me?.score)
    }

    // ─── REPO-04: HTTP 401 → ApiException(401) ───────────────────────────────

    @Test
    fun `REPO-04 HTTP 401 response throws ApiException with code 401`() = runTest {
        val repo = buildRepo(FakeRankingApiService(errorApiResponse(401)))

        val result = repo.getRanking("charm", "day")

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertNotNull(ex)
        assertTrue("Expected ApiException but got ${ex?.javaClass?.simpleName}", ex is ApiException)
        assertEquals(401, (ex as ApiException).code)
    }
}
