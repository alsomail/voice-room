package com.voice.room.android.feature.profile

import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.user.UserProfile
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — ProfileViewModel (T-30024)
 *
 * PC-01: init → Loading → Success，profile 字段与 repository 返回一致
 * PC-02: Success.fromCache = false（网络正常首次加载）
 * PC-05: copyId 发出 ShowToast("ID 已复制")
 * PC-08: logout → clearToken() + NavigateToLogin 事件
 * PC-09: coinBalance 正确携带
 * PC-10: IOException 无缓存 → Error 状态
 * PC-11: IOException 有缓存（先成功后失败）→ Success(fromCache=true) + ShowToast 含"缓存"
 * PC-13: 重试后 getMe() 被调用第 2 次，成功后 fromCache=false
 * PC-14: CancellationException 被 re-throw，不触发 ShowToast 或 Error
 * PC-16: coinBalance=0 正常展示，不崩溃
 * PC-17: logout() 在 Loading 状态时可执行，不崩溃
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ProfileViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake UserRepository ──────────────────────────────────────────────────

    /**
     * 可配置的 FakeUserRepository：
     * - [result]：getMe() 返回值，测试中可替换
     * - [throwCancellation]：为 true 时 getMe() 抛出 CancellationException
     */
    private class FakeUserRepository(
        var result: Result<UserProfile> = Result.success(DEFAULT_PROFILE),
        private val throwCancellation: Boolean = false,
    ) : IUserRepository {
        var getCallCount = 0

        override suspend fun getMe(): Result<UserProfile> {
            getCallCount++
            if (throwCancellation) throw CancellationException("Coroutine cancelled")
            return result
        }

        companion object {
            val DEFAULT_PROFILE = UserProfile(
                id = "u001",
                phone = "+966512345678",
                nickname = "TestUser",
                avatar = null,
                coinBalance = 1000L,
                vipLevel = 0,
                createdAt = "2026-01-01T00:00:00Z",
            )
        }
    }

    // ─── Fake TokenManager ────────────────────────────────────────────────────

    private class FakeTokenManager(
        private var token: String? = "valid.jwt.token",
    ) : ITokenManager {
        override suspend fun saveToken(token: String) { this.token = token }
        override suspend fun getToken(): String? = token
        override suspend fun clearToken() { token = null }
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    private fun buildViewModel(
        userRepository: IUserRepository = FakeUserRepository(),
        tokenManager: ITokenManager = FakeTokenManager(),
    ) = ProfileViewModel(userRepository, tokenManager)

    // ─── PC-01 / PC-02: init → Success, fromCache=false ──────────────────────

    @Test
    fun `PC-01 init triggers loadProfile and uiState becomes Success with correct profile`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository()

            // Collect states
            val states = mutableListOf<ProfileUiState>()
            val vm = ProfileViewModel(fakeRepo, FakeTokenManager())
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.uiState.collect { states.add(it) }
            }

            advanceUntilIdle()
            collectJob.cancel()

            // Should transition Loading → Success
            assertTrue(
                "First state must be Loading, got: ${states.firstOrNull()}",
                states.first() is ProfileUiState.Loading
            )
            val success = states.last() as? ProfileUiState.Success
            assertTrue("Last state must be Success, got: ${states.last()}", success != null)
            assertEquals(FakeUserRepository.DEFAULT_PROFILE, success!!.profile)
        }

    @Test
    fun `PC-02 first load sets fromCache=false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            collectJob.cancel()

            val state = vm.uiState.value as ProfileUiState.Success
            assertEquals(false, state.fromCache)
        }

    // ─── PC-09: coinBalance 正确携带 ──────────────────────────────────────────

    @Test
    fun `PC-09 coinBalance is correctly set in Success state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val profile = FakeUserRepository.DEFAULT_PROFILE.copy(coinBalance = 9876L)
            val vm = buildViewModel(userRepository = FakeUserRepository(result = Result.success(profile)))
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val state = vm.uiState.value as ProfileUiState.Success
            assertEquals(9876L, state.profile.coinBalance)
        }

    // ─── PC-05: copyId → ShowToast("ID 已复制") ──────────────────────────────

    @Test
    fun `PC-05 copyId emits ShowToast with ID copied message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()

            val events = mutableListOf<ProfileEvent>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            vm.copyId("u001")
            advanceUntilIdle()
            collectJob.cancel()

            assertTrue(
                "Should emit ShowToast, got: $events",
                events.any { it is ProfileEvent.ShowToast && it.message == "ID 已复制" }
            )
        }

    // ─── PC-08: logout → clearToken + NavigateToLogin ────────────────────────

    @Test
    fun `PC-08 logout clears token and emits NavigateToLogin`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val tokenManager = FakeTokenManager()
            val vm = buildViewModel(tokenManager = tokenManager)

            val events = mutableListOf<ProfileEvent>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            vm.logout()
            advanceUntilIdle()
            collectJob.cancel()

            assertNull("Token should be cleared after logout", tokenManager.getToken())
            assertTrue(
                "Should emit NavigateToLogin, got: $events",
                events.contains(ProfileEvent.NavigateToLogin)
            )
        }

    // ─── PC-10: IOException 无缓存 → Error ────────────────────────────────────

    @Test
    fun `PC-10 IOException without cache transitions to Error state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository(
                result = Result.failure(IOException("No network"))
            )
            val vm = buildViewModel(userRepository = fakeRepo)
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val state = vm.uiState.value
            assertTrue("Should be Error, got: $state", state is ProfileUiState.Error)
        }

    @Test
    fun `PC-10 Error state contains message from exception`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository(
                result = Result.failure(IOException("Network unavailable"))
            )
            val vm = buildViewModel(userRepository = fakeRepo)
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val state = vm.uiState.value as ProfileUiState.Error
            assertEquals("Network unavailable", state.message)
        }

    // ─── PC-11: IOException 有缓存 → Success(fromCache=true) + ShowToast ──────

    @Test
    fun `PC-11 IOException with cached data shows cached Success and ShowToast`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository(result = Result.success(FakeUserRepository.DEFAULT_PROFILE))
            val vm = buildViewModel(userRepository = fakeRepo)

            // First load: success → populates cache
            val stateJob = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            // Now simulate network failure
            val events = mutableListOf<ProfileEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }
            fakeRepo.result = Result.failure(IOException("Network down"))
            vm.loadProfile()
            advanceUntilIdle()

            stateJob.cancel()
            eventsJob.cancel()

            val state = vm.uiState.value as? ProfileUiState.Success
            assertTrue("Should be Success(fromCache=true), got: ${vm.uiState.value}", state != null)
            assertTrue("Should be fromCache=true", state!!.fromCache)
            assertEquals(FakeUserRepository.DEFAULT_PROFILE, state.profile)
            assertTrue(
                "Should emit ShowToast with cache-related message, got: $events",
                events.any { it is ProfileEvent.ShowToast && it.message.contains("缓存") }
            )
        }

    // ─── PC-13: 重试后 getMe() 被调用第 2 次，成功后 fromCache=false ────────────

    @Test
    fun `PC-13 retry after error calls getMe again and succeeds with fromCache=false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository(result = Result.failure(IOException("Network down")))
            val vm = buildViewModel(userRepository = fakeRepo)
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            // Simulate recovery: fix the repo and retry
            fakeRepo.result = Result.success(FakeUserRepository.DEFAULT_PROFILE)
            vm.loadProfile()
            advanceUntilIdle()
            job.cancel()

            // getMe should have been called at least twice (init + retry)
            assertTrue("getMe should be called >= 2 times", fakeRepo.getCallCount >= 2)
            val state = vm.uiState.value as? ProfileUiState.Success
            assertTrue("Should be Success after retry, got: ${vm.uiState.value}", state != null)
            assertEquals(false, state!!.fromCache)
        }

    // ─── PC-14: CancellationException re-throw ────────────────────────────────

    @Test
    fun `PC-14 CancellationException is rethrown and does not emit ShowToast or Error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeUserRepository(throwCancellation = true)
            val vm = ProfileViewModel(fakeRepo, FakeTokenManager())

            val events = mutableListOf<ProfileEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            // Run loadProfile — CancellationException should cancel the coroutine
            // The viewModelScope.launch will catch CancellationException by rethrow semantics
            advanceUntilIdle()
            eventsJob.cancel()

            // No ShowToast events should be emitted (CancellationException not swallowed)
            assertTrue(
                "CancellationException must not emit ShowToast events, got: $events",
                events.none { it is ProfileEvent.ShowToast }
            )
            // State should remain Loading (the coroutine was cancelled before emitting Error)
            // OR no Error state should be present
            val state = vm.uiState.value
            assertTrue(
                "State should be Loading (not Error) when CancellationException is rethrown, got: $state",
                state !is ProfileUiState.Error
            )
        }

    // ─── PC-16: coinBalance=0 正常 ────────────────────────────────────────────

    @Test
    fun `PC-16 coinBalance zero does not crash and is correctly reflected`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val profile = FakeUserRepository.DEFAULT_PROFILE.copy(coinBalance = 0L)
            val vm = buildViewModel(userRepository = FakeUserRepository(result = Result.success(profile)))
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val state = vm.uiState.value as ProfileUiState.Success
            assertEquals(0L, state.profile.coinBalance)
        }

    // ─── PC-17: logout() 在 Loading 状态时不崩溃 ──────────────────────────────

    @Test
    fun `PC-17 logout in Loading state does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // Use a repo that never returns to keep state Loading
            val neverReturningRepo = object : IUserRepository {
                override suspend fun getMe(): Result<UserProfile> {
                    // Suspend indefinitely
                    kotlinx.coroutines.delay(Long.MAX_VALUE)
                    return Result.success(FakeUserRepository.DEFAULT_PROFILE)
                }
            }
            val tokenManager = FakeTokenManager()
            val vm = ProfileViewModel(neverReturningRepo, tokenManager)

            // ViewModel is now in Loading state; call logout()
            val events = mutableListOf<ProfileEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            vm.logout() // Should not crash
            advanceUntilIdle()
            eventsJob.cancel()

            // Token should be cleared and NavigateToLogin emitted
            assertNull("Token should be cleared", tokenManager.getToken())
            assertTrue(
                "NavigateToLogin should be emitted",
                events.contains(ProfileEvent.NavigateToLogin)
            )
        }

    // ─── Avatar null 边界（PC-15 ViewModel 层验证）─────────────────────────────

    @Test
    fun `PC-15 avatar null does not crash ViewModel`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val profile = FakeUserRepository.DEFAULT_PROFILE.copy(avatar = null)
            val vm = buildViewModel(userRepository = FakeUserRepository(result = Result.success(profile)))
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val state = vm.uiState.value as ProfileUiState.Success
            assertNull("avatar should be null in success state", state.profile.avatar)
        }
}
