package com.voice.room.android.core.network

import com.voice.room.android.domain.local.ITokenManager
import okhttp3.Call
import okhttp3.Connection
import okhttp3.Interceptor
import okhttp3.Protocol
import okhttp3.Request
import okhttp3.Response
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.TimeUnit

/**
 * 单元测试：AuthInterceptor
 *
 * 验证用例：
 * 1. 有 token → Authorization: Bearer xxx header 自动注入
 * 2. 无 token → 不注入 Authorization header（匿名请求）
 * 3. token 为空字符串 → 不注入（与无 token 同等处理）
 * 4. 401 响应 → unauthorizedHandler.onUnauthorized() 被调用
 * 5. 200 响应 → handler 不触发
 * 6. 500 响应 → handler 不触发
 * 7. 401 响应 → proceed 只调用一次（无重试）
 */
class AuthInterceptorTest {

    // ─── Fakes ───────────────────────────────────────────────────────────────

    /** 可配置 token 的假 TokenManager，记录 clearToken 是否被调用 */
    private class FakeTokenManager(private val storedToken: String? = null) : ITokenManager {
        var clearTokenCalled = false

        override suspend fun saveToken(token: String) {}
        override suspend fun getToken(): String? = storedToken
        override suspend fun clearToken() { clearTokenCalled = true }
    }

    /** 记录 onUnauthorized 是否被调用 */
    private class FakeUnauthorizedHandler : UnauthorizedHandler {
        var onUnauthorizedCalled = false

        override suspend fun onUnauthorized() {
            onUnauthorizedCalled = true
        }

        override fun resetUnauthorized() {
            // no-op in test fake
        }
    }

    /**
     * 可控制响应码的假 Chain：
     * - [proceedRequest]  暴露 proceed() 实际收到的 Request（用于断言 Header）
     * - [proceedCallCount] 记录 proceed 调用次数（用于断言不重试）
     */
    private class FakeChain(
        private val originalRequest: Request,
        private val responseCode: Int = 200
    ) : Interceptor.Chain {

        var proceedRequest: Request? = null
            private set

        var proceedCallCount: Int = 0
            private set

        override fun request(): Request = originalRequest

        override fun proceed(request: Request): Response {
            proceedRequest = request
            proceedCallCount++
            return Response.Builder()
                .request(request)
                .protocol(Protocol.HTTP_1_1)
                .code(responseCode)
                .message(if (responseCode == 200) "OK" else "")
                .build()
        }

        // ── 以下方法在本测试场景中不使用 ─────────────────────────────────────
        override fun connection(): Connection? = null
        override fun call(): Call = throw UnsupportedOperationException("not used in tests")
        override fun connectTimeoutMillis(): Int = 0
        override fun withConnectTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
        override fun readTimeoutMillis(): Int = 0
        override fun withReadTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
        override fun writeTimeoutMillis(): Int = 0
        override fun withWriteTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
    }

    // ─── 辅助工厂 ────────────────────────────────────────────────────────────

    private fun buildRequest(url: String = "https://example.com/api/test"): Request =
        Request.Builder().url(url).build()

    // ─── 测试用例 ─────────────────────────────────────────────────────────────

    @Test
    fun `when token exists, request includes Authorization Bearer header`() {
        val tokenManager = FakeTokenManager(storedToken = "my-jwt-token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest())

        interceptor.intercept(chain)

        val forwarded = checkNotNull(chain.proceedRequest) { "proceed() was not called" }
        assertEquals("Bearer my-jwt-token", forwarded.header("Authorization"))
    }

    @Test
    fun `when no token, request has no Authorization header`() {
        val tokenManager = FakeTokenManager(storedToken = null)
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest())

        interceptor.intercept(chain)

        val forwarded = checkNotNull(chain.proceedRequest)
        assertNull(forwarded.header("Authorization"))
    }

    @Test
    fun `when token is blank string, request has no Authorization header`() {
        val tokenManager = FakeTokenManager(storedToken = "")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest())

        interceptor.intercept(chain)

        val forwarded = checkNotNull(chain.proceedRequest)
        assertNull(forwarded.header("Authorization"))
    }

    @Test
    fun `when 401 response, unauthorized handler is invoked`() {
        val tokenManager = FakeTokenManager(storedToken = "token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 401)

        interceptor.intercept(chain)

        assertTrue("onUnauthorized() should have been called on 401", handler.onUnauthorizedCalled)
    }

    @Test
    fun `when 200 response, unauthorized handler is not invoked`() {
        val tokenManager = FakeTokenManager(storedToken = "token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 200)

        interceptor.intercept(chain)

        assertFalse("onUnauthorized() must NOT be called on 200", handler.onUnauthorizedCalled)
    }

    @Test
    fun `when 403 response, unauthorized handler is not invoked`() {
        val tokenManager = FakeTokenManager(storedToken = "token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 403)

        interceptor.intercept(chain)

        assertFalse("onUnauthorized() must NOT be called on 403", handler.onUnauthorizedCalled)
    }

    @Test
    fun `when 500 response, unauthorized handler is not invoked`() {
        val tokenManager = FakeTokenManager(storedToken = "token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 500)

        interceptor.intercept(chain)

        assertFalse("onUnauthorized() must NOT be called on 500", handler.onUnauthorizedCalled)
    }

    @Test
    fun `when 401 response, request is not retried (proceed called exactly once)`() {
        val tokenManager = FakeTokenManager(storedToken = "token")
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 401)

        interceptor.intercept(chain)

        assertEquals(
            "proceed() must be called exactly once to prevent infinite retry loop",
            1,
            chain.proceedCallCount
        )
    }

    @Test
    fun `when 401 and no token, unauthorized handler is still invoked`() {
        val tokenManager = FakeTokenManager(storedToken = null)
        val handler = FakeUnauthorizedHandler()
        val interceptor = AuthInterceptor(tokenManager, handler)
        val chain = FakeChain(buildRequest(), responseCode = 401)

        interceptor.intercept(chain)

        assertTrue("Should still handle 401 even when there was no token", handler.onUnauthorizedCalled)
    }
}
