package com.voice.room.android.data.gift

import com.voice.room.android.data.remote.api.GiftApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.GiftDto
import kotlinx.coroutines.delay
import kotlinx.coroutines.joinAll
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Response
import java.io.IOException
import kotlinx.coroutines.ExperimentalCoroutinesApi

/**
 * TDD 单元测试 — RetrofitGiftRepository (T-30028 R1 修复)
 *
 * R01: HTTP 200 + code=0 → Result.success(List<GiftVO>)，字段完整映射
 * R02: 网络 IOException → Result.failure(IOException)
 * R03: HTTP 500，error body → Result.failure(ApiException)
 * R04: 有效缓存（<60s）→ 不再发起 HTTP 请求
 * R05: 缓存过期（>60s）→ 重新发起 HTTP 请求
 * MEDIUM-1: 两个协程并发调用 listGifts() → API 仅被调用一次（Mutex 防 TOCTOU）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RetrofitGiftRepositoryTest {

    // ─── Fake GiftApiService ──────────────────────────────────────────────────

    /** 可配置的 Fake API，记录调用次数，支持可控延迟 */
    private class FakeGiftApiService(
        private val responseProvider: suspend () -> Response<ApiResponse<List<GiftDto>>>,
        private val delayMs: Long = 0L,
    ) : GiftApiService {
        var callCount = 0

        override suspend fun listGifts(
            acceptLanguage: String,
        ): Response<ApiResponse<List<GiftDto>>> {
            callCount++
            if (delayMs > 0L) delay(delayMs)
            return responseProvider()
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun makeGiftDto(id: String = "g1") = GiftDto(
        id = id,
        code = "code_$id",
        name = "礼物$id",
        iconUrl = "https://cdn.example.com/$id.png",
        price = 10L,
        sortOrder = 1,
        tier = 2,
    )

    private fun successResponse(
        dtos: List<GiftDto> = listOf(makeGiftDto()),
    ): Response<ApiResponse<List<GiftDto>>> =
        Response.success(
            ApiResponse(code = 0, message = "ok", data = dtos, requestId = "req-001")
        )

    private fun errorResponse(
        httpCode: Int,
        errorJson: String = """{"code":$httpCode,"message":"error","request_id":"req-err"}""",
    ): Response<ApiResponse<List<GiftDto>>> =
        Response.error(httpCode, errorJson.toResponseBody())

    // ─── Tests ────────────────────────────────────────────────────────────────

    // --- R01: 成功响应映射 ---

    @Test
    fun `R01 HTTP 200 code 0 returns success with mapped GiftVO list`() = runTest {
        val dto = makeGiftDto("unicorn-1").copy(name = "独角兽", price = 66L, tier = 3)
        val fakeApi = FakeGiftApiService(responseProvider = { successResponse(listOf(dto)) })
        val repo = RetrofitGiftRepository(fakeApi)

        val result = repo.listGifts("en")

        assertTrue("result should be success", result.isSuccess)
        val gifts = result.getOrThrow()
        assertEquals("should return 1 gift", 1, gifts.size)
        val gift = gifts[0]
        assertEquals("unicorn-1", gift.id)
        assertEquals("独角兽", gift.name)
        assertEquals(66L, gift.price)
        assertEquals(3, gift.tier)
    }

    // --- R02: 网络错误 ---

    @Test
    fun `R02 network IOException returns failure`() = runTest {
        val fakeApi = FakeGiftApiService(
            responseProvider = { throw IOException("Network error") }
        )
        val repo = RetrofitGiftRepository(fakeApi)

        val result = repo.listGifts("en")

        assertTrue("result should be failure", result.isFailure)
        assertTrue("failure cause should be IOException",
            result.exceptionOrNull() is IOException)
    }

    // --- R03: HTTP 错误响应 ---

    @Test
    fun `R03 HTTP 500 with error body returns ApiException failure`() = runTest {
        val fakeApi = FakeGiftApiService(
            responseProvider = {
                errorResponse(500, """{"code":50001,"message":"Server Error","request_id":"e1"}""")
            }
        )
        val repo = RetrofitGiftRepository(fakeApi)

        val result = repo.listGifts("en")

        assertTrue("result should be failure", result.isFailure)
        assertNotNull("exception should not be null", result.exceptionOrNull())
    }

    // --- R04: 有效缓存命中，不重复请求 ---

    @Test
    fun `R04 valid cache returns cached result without additional API call`() = runTest {
        val fakeApi = FakeGiftApiService(responseProvider = { successResponse() })
        val repo = RetrofitGiftRepository(fakeApi)

        repo.listGifts("en")                          // 首次调用：缓存写入
        val callCountAfterFirst = fakeApi.callCount

        repo.listGifts("en")                          // 第二次调用：应命中缓存
        val callCountAfterSecond = fakeApi.callCount

        assertEquals("first call should make API request", 1, callCountAfterFirst)
        assertEquals("second call should use cache, not call API again",
            1, callCountAfterSecond)
    }

    // --- R05: 缓存过期，重新请求 ---

    @Test
    fun `R05 expired cache triggers new API call`() = runTest {
        val fakeApi = FakeGiftApiService(responseProvider = { successResponse() })

        // 注入 cacheDurationMs = 0ms，使缓存立即过期
        val repo = RetrofitGiftRepository(fakeApi, cacheDurationMs = 0L)

        repo.listGifts("en")                          // 首次调用
        repo.listGifts("en")                          // 第二次调用：缓存已过期

        assertEquals("both calls should hit API when cache expires", 2, fakeApi.callCount)
    }

    // --- MEDIUM-1: 并发调用只发起一次 HTTP 请求（Mutex 防 TOCTOU 竞态）---

    /**
     * 复现 TOCTOU 竞态：
     * 1. 两个协程并发调用 listGifts()，缓存为空
     * 2. delay(100ms) 制造竞争窗口：两者均通过缓存检查后挂起，等待 IO 完成
     * 3. 期望：有 Mutex 保护 → API 仅被调用一次
     *
     * 修复前（@Volatile 无原子性保障）：callCount = 2 → 测试 FAIL
     * 修复后（Mutex.withLock 包裹 check-then-act）：callCount = 1 → 测试 PASS
     */
    @Test
    fun `MEDIUM-1 concurrent listGifts with empty cache only calls API once`() = runTest {
        // 100ms 虚拟延迟 = 竞争窗口，确保两个协程均能"通过缓存检查后挂起"
        val fakeApi = FakeGiftApiService(
            responseProvider = { successResponse() },
            delayMs = 100L,
        )
        val repo = RetrofitGiftRepository(fakeApi)

        // 并发启动两个协程（StandardTestDispatcher：launch 只调度，不立即执行）
        val job1 = launch { repo.listGifts("en") }
        val job2 = launch { repo.listGifts("en") }

        // advanceUntilIdle 推进虚拟时间，跑完所有挂起任务
        advanceUntilIdle()
        joinAll(job1, job2)

        // 核心断言：Mutex 保护下只发起一次网络请求，第二个协程命中缓存直接返回
        assertEquals(
            "Mutex must prevent duplicate API calls: expected 1 call but got ${fakeApi.callCount}",
            1,
            fakeApi.callCount,
        )
    }

    // --- 额外：locale 正确传递 ---

    @Test
    fun `locale ar is passed to API Accept-Language header`() = runTest {
        var capturedLocale: String? = null
        val fakeApi = object : GiftApiService {
            override suspend fun listGifts(
                acceptLanguage: String,
            ): Response<ApiResponse<List<GiftDto>>> {
                capturedLocale = acceptLanguage
                return Response.success(
                    ApiResponse(code = 0, message = "ok", data = emptyList(), requestId = "1")
                )
            }
        }
        val repo = RetrofitGiftRepository(fakeApi)

        repo.listGifts("ar")

        assertEquals("locale should be passed as Accept-Language", "ar", capturedLocale)
    }

    // --- 额外：空列表不崩溃 ---

    @Test
    fun `empty gift list from API returns success with empty list`() = runTest {
        val fakeApi = FakeGiftApiService(responseProvider = { successResponse(emptyList()) })
        val repo = RetrofitGiftRepository(fakeApi)

        val result = repo.listGifts("en")

        assertTrue("result should be success", result.isSuccess)
        assertTrue("gift list should be empty", result.getOrThrow().isEmpty())
    }
}
