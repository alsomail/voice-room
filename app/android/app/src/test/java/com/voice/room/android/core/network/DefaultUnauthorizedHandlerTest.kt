package com.voice.room.android.core.network

import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 单元测试：DefaultUnauthorizedHandler
 *
 * 验证用例：
 * 1. onUnauthorized() 调用 tokenManager.clearToken()（清除本地 Token）
 * 2. onUnauthorized() 向 unauthorizedEvent 发射事件（触发跳转登录页）
 * 3. 并发多次 onUnauthorized() 只触发一次事件（AtomicBoolean 保护）
 * 4. resetUnauthorized() 后，onUnauthorized() 可再次触发事件（模拟重新登录后恢复）
 * 5. clearToken() 在 event 发射前被调用
 */
@OptIn(ExperimentalCoroutinesApi::class)
class DefaultUnauthorizedHandlerTest {

    // ─── Fake ────────────────────────────────────────────────────────────────

    private class FakeTokenManager : ITokenManager {
        var clearTokenCallCount = 0

        override suspend fun saveToken(token: String) {}
        override suspend fun getToken(): String? = null
        override suspend fun clearToken() { clearTokenCallCount++ }
    }

    // ─── 测试用例 ─────────────────────────────────────────────────────────────

    @Test
    fun `onUnauthorized clears the stored token`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        handler.onUnauthorized()

        assertEquals(
            "clearToken() should be called once",
            1,
            tokenManager.clearTokenCallCount
        )
    }

    @Test
    fun `onUnauthorized emits to unauthorizedEvent (triggers navigate-to-login)`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        val receivedEvents = mutableListOf<Unit>()
        val collector = launch {
            handler.unauthorizedEvent.collect { receivedEvents.add(it) }
        }
        runCurrent() // Allow the collector coroutine to start and subscribe first

        handler.onUnauthorized()
        runCurrent() // Process the emitted event

        collector.cancel()

        assertEquals("unauthorizedEvent should emit exactly once", 1, receivedEvents.size)
    }

    /**
     * H-02 修复：多次串行调用 onUnauthorized() 只应触发 1 次事件。
     * 原测试断言 2 个事件——验证的是竞态 Bug，现修正为期望 1 个事件。
     */
    @Test
    fun `multiple onUnauthorized calls each emit one event`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        val receivedEvents = mutableListOf<Unit>()
        val collector = launch {
            handler.unauthorizedEvent.collect { receivedEvents.add(it) }
        }
        runCurrent() // Allow the collector coroutine to start and subscribe first

        handler.onUnauthorized()
        runCurrent()
        handler.onUnauthorized() // 第二次调用：AtomicBoolean 保护，应被忽略
        runCurrent()

        collector.cancel()

        assertEquals(
            "Concurrent/repeated onUnauthorized() calls must only produce 1 event (AtomicBoolean guard)",
            1,
            receivedEvents.size
        )
        assertEquals(
            "clearToken() must only be called once despite repeated onUnauthorized() calls",
            1,
            tokenManager.clearTokenCallCount
        )
    }

    /**
     * H-02 新增：5 个协程并发调用 onUnauthorized()，断言只触发 1 次事件、clearToken() 只调用 1 次。
     */
    @Test
    fun `concurrent onUnauthorized calls trigger exactly once`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        val receivedEvents = mutableListOf<Unit>()
        val collector = launch {
            handler.unauthorizedEvent.collect { receivedEvents.add(it) }
        }
        runCurrent()

        // 模拟多个并发 OkHttp 线程同时触发 401
        val jobs = (1..5).map {
            launch(Dispatchers.Default) { handler.onUnauthorized() }
        }
        jobs.forEach { it.join() }
        runCurrent()
        collector.cancel()

        assertEquals(
            "Concurrent 401s must trigger logout exactly once",
            1,
            receivedEvents.size
        )
        assertEquals(
            "clearToken() must be called exactly once",
            1,
            tokenManager.clearTokenCallCount
        )
    }

    /**
     * H-02 新增：resetUnauthorized() 重置 handled 标记后，onUnauthorized() 可再次触发——
     * 模拟用户重新登录（saveToken 成功）后，下一次 Token 失效仍能正常触发登出流程。
     */
    @Test
    fun `after resetUnauthorized, onUnauthorized can trigger again`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        val receivedEvents = mutableListOf<Unit>()
        val collector = launch {
            handler.unauthorizedEvent.collect { receivedEvents.add(it) }
        }
        runCurrent()

        // 第一次 401：正常触发
        handler.onUnauthorized()
        runCurrent()

        // 未重置时：第二次调用应被忽略
        handler.onUnauthorized()
        runCurrent()

        // 模拟用户重新登录成功（saveToken 后调用 resetUnauthorized）
        handler.resetUnauthorized()

        // 重置后再次 401：应再次触发
        handler.onUnauthorized()
        runCurrent()

        collector.cancel()

        assertEquals(
            "Should emit exactly twice: once before reset, once after reset",
            2,
            receivedEvents.size
        )
        assertEquals(
            "clearToken() should be called twice (first 401 + post-reset 401)",
            2,
            tokenManager.clearTokenCallCount
        )
    }

    @Test
    fun `onUnauthorized clears token before emitting event`() = runTest {
        val tokenManager = FakeTokenManager()
        val handler = DefaultUnauthorizedHandler(tokenManager)

        var clearCalledBeforeEmit = false
        val collector = launch {
            handler.unauthorizedEvent.collect {
                // 当 event 被发射时，clearToken 应该已经被调用
                clearCalledBeforeEmit = tokenManager.clearTokenCallCount > 0
            }
        }
        runCurrent() // Allow the collector coroutine to start and subscribe first

        handler.onUnauthorized()
        runCurrent()

        collector.cancel()

        assertTrue(
            "clearToken() must be called before the event is emitted",
            clearCalledBeforeEmit
        )
    }
}
