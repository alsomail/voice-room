package com.voice.room.android.data.user

import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.UserApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.UserMeResponseData
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Response
import java.io.IOException

/**
 * TDD 单元测试 — RetrofitUserRepository
 *
 * 覆盖范围：
 * 1. [正向] 成功响应 → Result.success(UserProfile)，所有字段正确映射
 * 2. [正向] avatar 为 null → UserProfile.avatar == null
 * 3. [正向] coinBalance 为 Long.MAX_VALUE → 正确映射（Long 边界）
 * 4. [正向] coinBalance=0, vipLevel=0 → 新用户默认值正确映射
 * 5. [异常] HTTP 401 → Result.failure(ApiException(40101))
 * 6. [异常] HTTP 403 → Result.failure(ApiException)
 * 7. [异常] HTTP 500 → Result.failure(ApiException)
 * 8. [异常] HTTP 4xx，无可解析 body → Result.failure(ApiException) 使用 HTTP 状态码
 * 9. [异常] 网络 IOException → Result.failure(IOException)
 * 10. [异常] 响应 body 为 null → Result.failure(ApiException)
 * 11. [异常] API code ≠ 0（2xx 但业务失败）→ Result.failure(ApiException)
 */
class RetrofitUserRepositoryTest {

    // ─────────────────────────────────────────────
    // Fakes
    // ─────────────────────────────────────────────

    /**
     * 可配置响应的假 UserApiService。
     * [responseProvider] 在 suspend 调用时执行，可以抛出异常模拟网络错误。
     */
    private class FakeUserApiService(
        private val responseProvider: suspend () -> Response<ApiResponse<UserMeResponseData>>
    ) : UserApiService {
        override suspend fun getMe(): Response<ApiResponse<UserMeResponseData>> = responseProvider()
    }

    // ─────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────

    private fun successResponse(data: UserMeResponseData): Response<ApiResponse<UserMeResponseData>> =
        Response.success(
            ApiResponse(code = 0, message = "ok", data = data, requestId = "req-test-001")
        )

    private fun errorResponse(
        httpCode: Int,
        errorJson: String = """{"code":$httpCode,"message":"HTTP $httpCode","request_id":"req-err"}"""
    ): Response<ApiResponse<UserMeResponseData>> =
        Response.error(httpCode, errorJson.toResponseBody())

    private val sampleData = UserMeResponseData(
        id = "user-uuid-001",
        phone = "+966512345678",
        nickname = "User_a1b2",
        avatar = "https://cdn.example.com/avatars/xxx.jpg",
        coinBalance = 1000L,
        vipLevel = 2,
        createdAt = "2026-04-17T00:00:00Z"
    )

    // ─────────────────────────────────────────────
    // 1. 正向：成功响应，完整字段映射
    // ─────────────────────────────────────────────

    @Test
    fun `getMe success returns Result success with all fields correctly mapped`() = runTest {
        val service = FakeUserApiService { successResponse(sampleData) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue("Expected success", result.isSuccess)
        val profile = result.getOrThrow()
        assertEquals("user-uuid-001", profile.id)
        assertEquals("+966512345678", profile.phone)
        assertEquals("User_a1b2", profile.nickname)
        assertEquals("https://cdn.example.com/avatars/xxx.jpg", profile.avatar)
        assertEquals(1000L, profile.coinBalance)
        assertEquals(2, profile.vipLevel)
        assertEquals("2026-04-17T00:00:00Z", profile.createdAt)
    }

    // ─────────────────────────────────────────────
    // 2. 正向：avatar 为 null
    // ─────────────────────────────────────────────

    @Test
    fun `getMe success with null avatar returns UserProfile with null avatar`() = runTest {
        val service = FakeUserApiService { successResponse(sampleData.copy(avatar = null)) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isSuccess)
        assertNull("avatar should be null", result.getOrThrow().avatar)
    }

    // ─────────────────────────────────────────────
    // 3. 正向：coinBalance 边界值 Long.MAX_VALUE
    // ─────────────────────────────────────────────

    @Test
    fun `getMe maps coinBalance as Long including Long MAX_VALUE`() = runTest {
        val service = FakeUserApiService {
            successResponse(sampleData.copy(coinBalance = Long.MAX_VALUE))
        }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isSuccess)
        assertEquals(Long.MAX_VALUE, result.getOrThrow().coinBalance)
    }

    // ─────────────────────────────────────────────
    // 4. 正向：新用户 coinBalance=0, vipLevel=0
    // ─────────────────────────────────────────────

    @Test
    fun `getMe maps zero coinBalance and vipLevel for new user`() = runTest {
        val service = FakeUserApiService {
            successResponse(sampleData.copy(coinBalance = 0L, vipLevel = 0))
        }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isSuccess)
        val profile = result.getOrThrow()
        assertEquals(0L, profile.coinBalance)
        assertEquals(0, profile.vipLevel)
    }

    // ─────────────────────────────────────────────
    // 5. 异常：HTTP 401 → ApiException(40101)
    // ─────────────────────────────────────────────

    @Test
    fun `getMe with HTTP 401 returns Result failure with ApiException code 40101`() = runTest {
        val errorJson = """{"code":40101,"message":"Unauthorized","request_id":"req-002"}"""
        val service = FakeUserApiService { errorResponse(401, errorJson) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertNotNull(ex)
        assertTrue("Expected ApiException, got ${ex?.javaClass?.simpleName}", ex is ApiException)
        assertEquals(40101, (ex as ApiException).code)
    }

    // ─────────────────────────────────────────────
    // 6. 异常：HTTP 403 → ApiException
    // ─────────────────────────────────────────────

    @Test
    fun `getMe with HTTP 403 returns Result failure with ApiException`() = runTest {
        val errorJson = """{"code":40301,"message":"Forbidden","request_id":"req-003"}"""
        val service = FakeUserApiService { errorResponse(403, errorJson) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertTrue("Expected ApiException", ex is ApiException)
        assertEquals(40301, (ex as ApiException).code)
    }

    // ─────────────────────────────────────────────
    // 7. 异常：HTTP 500 → ApiException
    // ─────────────────────────────────────────────

    @Test
    fun `getMe with HTTP 500 returns Result failure with ApiException`() = runTest {
        val errorJson = """{"code":50001,"message":"Internal Server Error","request_id":"req-004"}"""
        val service = FakeUserApiService { errorResponse(500, errorJson) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        assertTrue(result.exceptionOrNull() is ApiException)
    }

    // ─────────────────────────────────────────────
    // 8. 异常：HTTP 4xx，error body 无法解析 → 使用 HTTP 状态码回退
    // ─────────────────────────────────────────────

    @Test
    fun `getMe with HTTP 4xx and unparseable error body falls back to HTTP status code`() = runTest {
        val notJsonBody = "Not a JSON body"
        val service = FakeUserApiService { errorResponse(422, notJsonBody) }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertTrue("Expected ApiException", ex is ApiException)
        // 回退到 HTTP 状态码 422
        assertEquals(422, (ex as ApiException).code)
    }

    // ─────────────────────────────────────────────
    // 9. 异常：网络 IOException
    // ─────────────────────────────────────────────

    @Test
    fun `getMe when network throws IOException returns Result failure`() = runTest {
        val service = FakeUserApiService { throw IOException("Network unreachable") }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        assertTrue(
            "Expected IOException, got ${result.exceptionOrNull()?.javaClass?.simpleName}",
            result.exceptionOrNull() is IOException
        )
    }

    // ─────────────────────────────────────────────
    // 10. 异常：响应 body 为 null（HTTP 2xx 但无 body）
    // ─────────────────────────────────────────────

    @Test
    fun `getMe when 2xx response body is null returns Result failure`() = runTest {
        val service = FakeUserApiService {
            @Suppress("UNCHECKED_CAST")
            Response.success(null as ApiResponse<UserMeResponseData>?)
        }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        assertNotNull(result.exceptionOrNull())
    }

    // ─────────────────────────────────────────────
    // 11. 异常：API code ≠ 0（HTTP 200 但业务失败）
    // ─────────────────────────────────────────────

    @Test
    fun `getMe when API code is nonzero returns Result failure with ApiException`() = runTest {
        val service = FakeUserApiService {
            Response.success(
                ApiResponse(
                    code = 40101,
                    message = "Unauthorized",
                    data = null,
                    requestId = "req-005"
                )
            )
        }
        val repo = RetrofitUserRepository(service)

        val result = repo.getMe()

        assertTrue(result.isFailure)
        val ex = result.exceptionOrNull()
        assertTrue("Expected ApiException, got ${ex?.javaClass?.simpleName}", ex is ApiException)
        assertEquals(40101, (ex as ApiException).code)
    }
}
