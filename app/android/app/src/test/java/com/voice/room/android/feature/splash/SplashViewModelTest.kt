package com.voice.room.android.feature.splash

import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — SplashViewModel (T-30019)
 *
 * SP-01: 有效 token → NavigateToMain
 * SP-02: null token → NavigateToLogin
 * SP-03: 空字符串 token → NavigateToLogin
 * SP-04: 纯空白字符串 token → NavigateToLogin
 * SP-05: tokenManager 抛异常 → NavigateToLogin（异常安全）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class SplashViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake TokenManager ───────────────────────────────

    /**
     * 可配置的 FakeTokenManager：
     * - [token]：getToken() 返回值
     * - [shouldThrow]：为 true 时 getToken() 抛出 RuntimeException 模拟 DataStore 损坏
     * - [throwCancellation]：为 true 时 getToken() 抛出 CancellationException 模拟协程取消
     */
    private class FakeTokenManager(
        private var token: String? = null,
        private val shouldThrow: Boolean = false,
        private val throwCancellation: Boolean = false
    ) : ITokenManager {
        override suspend fun saveToken(token: String) {
            this.token = token
        }

        override suspend fun getToken(): String? {
            if (throwCancellation) throw CancellationException("Coroutine cancelled")
            if (shouldThrow) throw RuntimeException("DataStore corrupted")
            return token
        }

        override suspend fun clearToken() {
            token = null
        }
    }

    // ─── Helper ──────────────────────────────────────────

    /**
     * 创建 ViewModel 并收集 navEvent，执行 checkAuth()，返回所有收集到的事件。
     */
    private fun collectEventsAfterCheckAuth(
        tokenManager: ITokenManager
    ): List<SplashNavEvent> {
        val events = mutableListOf<SplashNavEvent>()
        // 使用 mainDispatcherRule.testDispatcher 确保 viewModelScope 和 runTest 共享同一调度器
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = SplashViewModel(tokenManager)
            val job = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.navEvent.collect { events.add(it) }
            }
            vm.checkAuth()
            advanceUntilIdle()
            job.cancel()
        }
        return events
    }

    // ─── SP-01: 有效 token → NavigateToMain ──────────────

    @Test
    fun `SP-01 checkAuth emits NavigateToMain when token is valid non-blank`() {
        val events = collectEventsAfterCheckAuth(
            FakeTokenManager(token = "valid.jwt.token")
        )
        assertEquals(
            "Should emit exactly one NavigateToMain event",
            listOf(SplashNavEvent.NavigateToMain),
            events
        )
    }

    // ─── SP-02: null token → NavigateToLogin ─────────────

    @Test
    fun `SP-02 checkAuth emits NavigateToLogin when token is null`() {
        val events = collectEventsAfterCheckAuth(
            FakeTokenManager(token = null)
        )
        assertEquals(
            "Should emit exactly one NavigateToLogin event",
            listOf(SplashNavEvent.NavigateToLogin),
            events
        )
    }

    // ─── SP-03: 空字符串 → NavigateToLogin ───────────────

    @Test
    fun `SP-03 checkAuth emits NavigateToLogin when token is empty string`() {
        val events = collectEventsAfterCheckAuth(
            FakeTokenManager(token = "")
        )
        assertEquals(
            "Empty string token should navigate to login",
            listOf(SplashNavEvent.NavigateToLogin),
            events
        )
    }

    // ─── SP-04: 纯空白字符串 → NavigateToLogin ──────────

    @Test
    fun `SP-04 checkAuth emits NavigateToLogin when token is blank whitespace`() {
        val events = collectEventsAfterCheckAuth(
            FakeTokenManager(token = "   ")
        )
        assertEquals(
            "Blank whitespace token should navigate to login",
            listOf(SplashNavEvent.NavigateToLogin),
            events
        )
    }

    // ─── SP-05: tokenManager 抛异常 → NavigateToLogin ───

    @Test
    fun `SP-05 checkAuth emits NavigateToLogin when tokenManager throws exception`() {
        val events = collectEventsAfterCheckAuth(
            FakeTokenManager(shouldThrow = true)
        )
        assertEquals(
            "Exception in tokenManager should fall back to NavigateToLogin",
            listOf(SplashNavEvent.NavigateToLogin),
            events
        )
    }

    // ─── SP-05b: CancellationException 不被吞噬 ─────────

    /**
     * 验证结构化并发规范：CancellationException 必须重新抛出，
     * 不能被 catch(_: Exception) 吞噬。
     *
     * 参考 RoomViewModel.kt 中的同类处理模式：
     *   catch (e: CancellationException) { throw e }
     *   catch (e: Exception) { ... }
     */
    @Test
    fun `SP-05b checkAuth rethrows CancellationException instead of swallowing it`() {
        val tokenManager = FakeTokenManager(throwCancellation = true)
        val vm = SplashViewModel(tokenManager)
        val events = mutableListOf<SplashNavEvent>()

        runTest(mainDispatcherRule.testDispatcher) {
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.navEvent.collect { events.add(it) }
            }

            vm.checkAuth()
            advanceUntilIdle()

            collectJob.cancel()
        }

        // CancellationException 应被重新抛出而非被 catch 吞噬。
        // 若被正确重抛，viewModelScope.launch 内的协程会取消退出，
        // 不会走到 emit(NavigateToLogin) 分支 → events 应为空。
        assertTrue(
            "CancellationException must not be swallowed — no nav event should be emitted, " +
                "but got: $events",
            events.isEmpty()
        )
    }
}
