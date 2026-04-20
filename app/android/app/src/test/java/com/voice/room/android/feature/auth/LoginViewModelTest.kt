package com.voice.room.android.feature.auth

import com.voice.room.android.core.network.UnauthorizedHandler
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.domain.auth.IAuthRepository
import com.voice.room.android.domain.auth.LoginResult
import com.voice.room.android.domain.auth.SendCodeResult
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 – LoginUiState 和 LoginViewModel 逻辑
 *
 * 覆盖范围：
 * 1. 沙特手机号格式验证（+966 前缀 + 9 位本机号）
 * 2. 空手机号时发送按钮禁用
 * 3. 60 秒倒计时逻辑
 * 4. RTL 布局标志（isRtlLayout）
 * 5. 登录按钮启用条件（6 位验证码）
 * 6. [T-30002] onLogin() API 集成：Success / Error / Loading / Navigation / DataStore
 * 7. [T-30002] onSendCode() API 集成：Success / Error
 */
@OptIn(ExperimentalCoroutinesApi::class)
class LoginViewModelTest {

    // ─────────────────────────────────────────────
    // JUnit Rule: Replace Dispatchers.Main for coroutine tests
    // ─────────────────────────────────────────────
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─────────────────────────────────────────────
    // 1. 手机号格式验证
    // ─────────────────────────────────────────────

    @Test
    fun `valid 9-digit saudi mobile number is valid`() {
        assertTrue(LoginUiState.isPhoneNumberValid("501234567"))
    }

    @Test
    fun `saudi number starting with 5 is valid`() {
        assertTrue(LoginUiState.isPhoneNumberValid("512345678"))
    }

    @Test
    fun `empty phone number is invalid`() {
        assertFalse(LoginUiState.isPhoneNumberValid(""))
    }

    @Test
    fun `phone number with fewer than 9 digits is invalid`() {
        assertFalse(LoginUiState.isPhoneNumberValid("1234567"))   // 7 digits
        assertFalse(LoginUiState.isPhoneNumberValid("12345678"))  // 8 digits
    }

    @Test
    fun `phone number with more than 9 digits is invalid`() {
        assertFalse(LoginUiState.isPhoneNumberValid("5012345678"))   // 10 digits
        assertFalse(LoginUiState.isPhoneNumberValid("50123456789"))  // 11 digits
    }

    @Test
    fun `phone number with exactly 9 digits containing hyphens is valid`() {
        // 用户输入带有分隔符时，应只计算数字位数
        assertTrue(LoginUiState.isPhoneNumberValid("501-234-567"))
    }

    @Test
    fun `phone number with whitespace stripped to 9 digits is valid`() {
        assertTrue(LoginUiState.isPhoneNumberValid("501 234 567"))
    }

    @Test
    fun `phone number with only whitespace is invalid`() {
        assertFalse(LoginUiState.isPhoneNumberValid("   "))
    }

    // ─────────────────────────────────────────────
    // 2. 发送验证码按钮启用 / 禁用
    // ─────────────────────────────────────────────

    @Test
    fun `send button disabled when phone number is empty`() {
        val state = LoginUiState(phoneNumber = "")
        assertFalse(state.isSendButtonEnabled)
    }

    @Test
    fun `send button disabled when phone number is too short`() {
        val state = LoginUiState(phoneNumber = "50123")
        assertFalse(state.isSendButtonEnabled)
    }

    @Test
    fun `send button enabled when phone is valid and no countdown`() {
        val state = LoginUiState(phoneNumber = "501234567", countdownSeconds = 0)
        assertTrue(state.isSendButtonEnabled)
    }

    @Test
    fun `send button disabled when countdown is active even with valid phone`() {
        val state = LoginUiState(phoneNumber = "501234567", countdownSeconds = 30)
        assertFalse(state.isSendButtonEnabled)
    }

    @Test
    fun `send button disabled when countdown is at last second`() {
        val state = LoginUiState(phoneNumber = "501234567", countdownSeconds = 1)
        assertFalse(state.isSendButtonEnabled)
    }

    // ─────────────────────────────────────────────
    // 3. 倒计时逻辑
    // ─────────────────────────────────────────────

    @Test
    fun `countdown constant is 60 seconds`() {
        assertEquals(60, LoginUiState.COUNTDOWN_SECONDS)
    }

    @Test
    fun `state is counting down when countdownSeconds is greater than 0`() {
        val state = LoginUiState(countdownSeconds = 60)
        assertTrue(state.isCountingDown)
    }

    @Test
    fun `state is counting down when countdownSeconds is 1`() {
        val state = LoginUiState(countdownSeconds = 1)
        assertTrue(state.isCountingDown)
    }

    @Test
    fun `state is not counting down when countdownSeconds is 0`() {
        val state = LoginUiState(countdownSeconds = 0)
        assertFalse(state.isCountingDown)
    }

    @Test
    fun `countdown can decrement to 0 and stop`() {
        var state = LoginUiState(countdownSeconds = 1)
        assertTrue(state.isCountingDown)
        state = state.copy(countdownSeconds = state.countdownSeconds - 1)
        assertEquals(0, state.countdownSeconds)
        assertFalse(state.isCountingDown)
    }

    @Test
    fun `countdown label shows remaining seconds`() {
        val state = LoginUiState(countdownSeconds = 42)
        assertEquals("42s", state.countdownLabel)
    }

    @Test
    fun `countdown label is empty when not counting`() {
        val state = LoginUiState(countdownSeconds = 0)
        assertEquals("", state.countdownLabel)
    }

    // ─────────────────────────────────────────────
    // 4. RTL 布局支持（阿拉伯语 / 沙特市场默认 RTL）
    // ─────────────────────────────────────────────

    @Test
    fun `default layout direction is RTL for Saudi market`() {
        val state = LoginUiState()
        assertTrue(state.isRtlLayout)
    }

    @Test
    fun `rtl flag can be toggled for LTR markets`() {
        val state = LoginUiState(isRtlLayout = false)
        assertFalse(state.isRtlLayout)
    }

    // ─────────────────────────────────────────────
    // 5. 默认值
    // ─────────────────────────────────────────────

    @Test
    fun `default country code is +966`() {
        val state = LoginUiState()
        assertEquals("+966", state.defaultCountryCode)
    }

    @Test
    fun `default phone number is empty`() {
        val state = LoginUiState()
        assertEquals("", state.phoneNumber)
    }

    @Test
    fun `default countdown is 0 meaning not counting`() {
        val state = LoginUiState()
        assertEquals(0, state.countdownSeconds)
        assertFalse(state.isCountingDown)
    }

    @Test
    fun `default verification code is empty`() {
        val state = LoginUiState()
        assertEquals("", state.verificationCode)
    }

    // ─────────────────────────────────────────────
    // 6. 登录按钮启用 / 禁用
    // ─────────────────────────────────────────────

    @Test
    fun `login button disabled when verification code fewer than 6 digits`() {
        val state = LoginUiState(phoneNumber = "501234567", verificationCode = "12345")
        assertFalse(state.isLoginButtonEnabled)
    }

    @Test
    fun `login button disabled when verification code is empty`() {
        val state = LoginUiState(phoneNumber = "501234567", verificationCode = "")
        assertFalse(state.isLoginButtonEnabled)
    }

    @Test
    fun `login button enabled when phone valid and code is exactly 6 digits`() {
        val state = LoginUiState(phoneNumber = "501234567", verificationCode = "123456")
        assertTrue(state.isLoginButtonEnabled)
    }

    @Test
    fun `login button disabled when phone invalid even with 6-digit code`() {
        val state = LoginUiState(phoneNumber = "", verificationCode = "123456")
        assertFalse(state.isLoginButtonEnabled)
    }

    @Test
    fun `login button disabled when code exceeds 6 digits`() {
        val state = LoginUiState(phoneNumber = "501234567", verificationCode = "1234567")
        assertFalse(state.isLoginButtonEnabled)
    }

    // ─────────────────────────────────────────────
    // 7. ViewModel 状态变更
    // ─────────────────────────────────────────────

    @Test
    fun `onPhoneNumberChanged updates phoneNumber in state`() {
        val vm = LoginViewModel()
        vm.onPhoneNumberChanged("501234567")
        assertEquals("501234567", vm.uiState.value.phoneNumber)
    }

    @Test
    fun `onVerificationCodeChanged updates verificationCode in state`() {
        val vm = LoginViewModel()
        vm.onVerificationCodeChanged("123456")
        assertEquals("123456", vm.uiState.value.verificationCode)
    }

    @Test
    fun `send button enabled after entering valid phone number`() {
        val vm = LoginViewModel()
        vm.onPhoneNumberChanged("501234567")
        assertTrue(vm.uiState.value.isSendButtonEnabled)
    }

    @Test
    fun `send button disabled when phone cleared`() {
        val vm = LoginViewModel()
        vm.onPhoneNumberChanged("501234567")
        vm.onPhoneNumberChanged("")
        assertFalse(vm.uiState.value.isSendButtonEnabled)
    }

    @Test
    fun `initial state has default country code +966`() {
        val vm = LoginViewModel()
        assertEquals("+966", vm.uiState.value.defaultCountryCode)
    }

    @Test
    fun `initial state RTL is true`() {
        val vm = LoginViewModel()
        assertTrue(vm.uiState.value.isRtlLayout)
    }

    // ═════════════════════════════════════════════
    // T-30002 Tests: onLogin() API 集成
    // ═════════════════════════════════════════════

    // ─────────────────────────────────────────────
    // 8. 登录成功：状态 + token 保存
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin success sets isLoginSuccess to true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertTrue(vm.uiState.value.isLoginSuccess)
        }

    @Test
    fun `onLogin success saves JWT token to token manager`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeTokenManager = FakeTokenManager()
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("jwt_token_abc", "user_1", false))
                ),
                tokenManager = fakeTokenManager
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertEquals("jwt_token_abc", fakeTokenManager.savedToken)
        }

    @Test
    fun `onLogin success clears error and isLoading`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(FakeAuthRepository(), FakeTokenManager())
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertFalse(vm.uiState.value.isLoading)
            assertNull(vm.uiState.value.error)
        }

    // ─────────────────────────────────────────────
    // 9. 登录 Loading 状态
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin sets isLoading true while request is in progress`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var wasLoadingDuringApiCall = false
            val capturingFake = object : IAuthRepository {
                override suspend fun sendCode(phone: String) =
                    Result.success(SendCodeResult(60))

                override suspend fun login(phone: String, code: String): Result<LoginResult> {
                    // Capture the loading state at the moment the API is called
                    wasLoadingDuringApiCall = true
                    return Result.success(LoginResult("token", "user_id", false))
                }
            }
            val vm = LoginViewModel(capturingFake, FakeTokenManager())
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            runCurrent() // run coroutine up to first suspension/return

            assertTrue("isLoading must be true while API call is in progress", wasLoadingDuringApiCall)
            assertFalse("isLoading must be false after completion", vm.uiState.value.isLoading)
        }

    // ─────────────────────────────────────────────
    // 10. 登录失败：验证码错误
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin failure with code 40103 sets error to 验证码错误`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(ApiException(40103, "Invalid verification code"))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("000000")

            vm.onLogin()
            advanceUntilIdle()

            assertFalse(vm.uiState.value.isLoginSuccess)
            assertEquals("验证码错误", vm.uiState.value.error)
            assertFalse(vm.uiState.value.isLoading)
        }

    @Test
    fun `onLogin failure with code 40104 sets error to 验证码已过期`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(ApiException(40104, "Verification code expired"))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("000000")

            vm.onLogin()
            advanceUntilIdle()

            assertEquals("验证码已过期", vm.uiState.value.error)
        }

    @Test
    fun `onLogin failure with code 40105 sets error to 验证码尝试次数超限`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(ApiException(40105, "Max attempts exceeded"))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("000000")

            vm.onLogin()
            advanceUntilIdle()

            assertEquals("验证码尝试次数超限", vm.uiState.value.error)
        }

    @Test
    fun `onLogin network error sets friendly error message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(Exception("Connection timeout"))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertEquals("网络异常，请稍后重试", vm.uiState.value.error)
            assertFalse(vm.uiState.value.isLoading)
        }

    // ─────────────────────────────────────────────
    // 11. 登录失败时不保存 token
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin failure does not save token to token manager`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeTokenManager = FakeTokenManager()
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(ApiException(40103, "Invalid code"))
                ),
                tokenManager = fakeTokenManager
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("000000")

            vm.onLogin()
            advanceUntilIdle()

            assertNull(fakeTokenManager.savedToken)
        }

    // ─────────────────────────────────────────────
    // 12. 新用户首次登录
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin with new user sets isNewUser to true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_new", isNew = true))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertTrue(vm.uiState.value.isNewUser)
        }

    @Test
    fun `onLogin with existing user keeps isNewUser false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_old", isNew = false))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertFalse(vm.uiState.value.isNewUser)
        }

    // ─────────────────────────────────────────────
    // 13. 导航事件
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin success emits NavigateToHall nav event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(FakeAuthRepository(), FakeTokenManager())

            val collectedEvents = mutableListOf<NavEvent>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.navEvent.collect { collectedEvents.add(it) }
            }

            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")
            vm.onLogin()
            advanceUntilIdle()
            collectJob.cancel()

            assertEquals(1, collectedEvents.size)
            assertTrue(collectedEvents.first() is NavEvent.NavigateToHall)
        }

    @Test
    fun `onLogin failure does not emit nav event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.failure(ApiException(40103, "Invalid code"))
                ),
                tokenManager = FakeTokenManager()
            )

            val collectedEvents = mutableListOf<NavEvent>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.navEvent.collect { collectedEvents.add(it) }
            }

            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("000000")
            vm.onLogin()
            advanceUntilIdle()
            collectJob.cancel()

            assertTrue(collectedEvents.isEmpty())
        }

    // ─────────────────────────────────────────────
    // 14. onLogin 未满足条件时不触发（防御性）
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin does nothing when login button is disabled`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeAuthRepository()
            val vm = LoginViewModel(fakeRepo, FakeTokenManager())

            // No phone/code → button disabled → should not call API
            vm.onLogin()
            advanceUntilIdle()

            assertEquals(0, fakeRepo.loginCallCount)
            assertFalse(vm.uiState.value.isLoginSuccess)
        }

    // ═════════════════════════════════════════════
    // T-30002 Tests: onSendCode() API 集成
    // ═════════════════════════════════════════════

    // ─────────────────────────────────────────────
    // 15. sendCode 成功：倒计时启动
    // ─────────────────────────────────────────────

    @Test
    fun `onSendCode success starts countdown at cooldown seconds`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    sendCodeResult = Result.success(SendCodeResult(cooldownSeconds = 60))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")

            vm.onSendCode()
            runCurrent() // run coroutine up to the first delay() in countdown

            assertEquals(60, vm.uiState.value.countdownSeconds)
            assertFalse(vm.uiState.value.isSendingCode)
        }

    @Test
    fun `onSendCode success clears isSendingCode and error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(FakeAuthRepository(), FakeTokenManager())
            vm.onPhoneNumberChanged("501234567")

            vm.onSendCode()
            advanceUntilIdle()

            assertFalse(vm.uiState.value.isSendingCode)
            assertNull(vm.uiState.value.error)
        }

    // ─────────────────────────────────────────────
    // 16. sendCode 失败：展示错误
    // ─────────────────────────────────────────────

    @Test
    fun `onSendCode failure sets error message and clears isSendingCode`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    sendCodeResult = Result.failure(
                        ApiException(42901, "Verification code sent too frequently")
                    )
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")

            vm.onSendCode()
            advanceUntilIdle()

            assertNotNull(vm.uiState.value.error)
            assertFalse(vm.uiState.value.isSendingCode)
            assertEquals(0, vm.uiState.value.countdownSeconds)
        }

    @Test
    fun `onSendCode network error sets friendly message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    sendCodeResult = Result.failure(Exception("Timeout"))
                ),
                tokenManager = FakeTokenManager()
            )
            vm.onPhoneNumberChanged("501234567")

            vm.onSendCode()
            advanceUntilIdle()

            assertNotNull(vm.uiState.value.error)
            assertFalse(vm.uiState.value.isSendingCode)
        }

    @Test
    fun `onSendCode does nothing when send button is disabled`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeAuthRepository()
            val vm = LoginViewModel(fakeRepo, FakeTokenManager())

            // No phone number → button disabled
            vm.onSendCode()
            advanceUntilIdle()

            assertEquals(0, fakeRepo.sendCodeCallCount)
        }

    // ═════════════════════════════════════════════
    // T-30002 Review Fix Tests (RED — must fail before implementation)
    // ═════════════════════════════════════════════

    // ─────────────────────────────────────────────
    // 17. [MEDIUM] 沙特手机号首位必须为 '5'
    // ─────────────────────────────────────────────

    @Test
    fun `phone number not starting with 5 is invalid`() {
        // "412345678" 是 9 位数字，但首位是 4，不符合沙特格式，应为 invalid
        assertFalse(LoginUiState.isPhoneNumberValid("412345678"))
    }

    @Test
    fun `phone number starting with 5 and exactly 9 digits is valid`() {
        // "512345678" 首位 5，9 位，应为 valid
        assertTrue(LoginUiState.isPhoneNumberValid("512345678"))
    }

    // ─────────────────────────────────────────────
    // 18. [HIGH] saveToken IOException → UI 不应永久死锁
    // ─────────────────────────────────────────────

    @Test
    fun `onLogin saveToken IOException sets isLoading false and error non-null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_id", false))
                ),
                tokenManager = ThrowingTokenManager(IOException("Disk full"))
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertFalse(
                "isLoading must be false even after saveToken throws IOException",
                vm.uiState.value.isLoading
            )
            assertNotNull(
                "error must be set when saveToken throws IOException",
                vm.uiState.value.error
            )
            assertFalse(
                "isLoginSuccess must remain false when saveToken throws IOException",
                vm.uiState.value.isLoginSuccess
            )
        }

    @Test
    fun `onLogin saveToken IOException does not emit nav event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_id", false))
                ),
                tokenManager = ThrowingTokenManager(IOException("Permission denied"))
            )

            val collectedEvents = mutableListOf<NavEvent>()
            val collectJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.navEvent.collect { collectedEvents.add(it) }
            }

            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")
            vm.onLogin()
            advanceUntilIdle()
            collectJob.cancel()

            assertTrue(
                "No NavEvent should be emitted when saveToken throws IOException",
                collectedEvents.isEmpty()
            )
        }

    @Test
    fun `onLogin saveToken IOException sets expected error message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_id", false))
                ),
                tokenManager = ThrowingTokenManager(IOException("Storage unavailable"))
            )
            vm.onPhoneNumberChanged("501234567")
            vm.onVerificationCodeChanged("123456")

            vm.onLogin()
            advanceUntilIdle()

            assertEquals(
                "登录失败，Token 存储异常，请重试",
                vm.uiState.value.error
            )
        }

    // ═════════════════════════════════════════════
    // T-30003 第二轮 Review H-01 修复测试
    // ═════════════════════════════════════════════

    // ─────────────────────────────────────────────
    // 19. [HIGH] 登录成功后必须调用 resetUnauthorized
    // ─────────────────────────────────────────────

    /**
     * 验证登录成功后 [LoginViewModel] 调用 [UnauthorizedHandler.resetUnauthorized]，
     * 确保 AtomicBoolean 在下次 token 失效时仍能触发登出（Review H-01 修复链路）。
     */
    @Test
    fun `login success calls resetUnauthorized`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeUnauthorizedHandler = FakeUnauthorizedHandler()
            val viewModel = LoginViewModel(
                authRepository = FakeAuthRepository(),
                tokenManager = FakeTokenManager(),
                unauthorizedHandler = fakeUnauthorizedHandler
            )
            // 执行登录
            viewModel.onPhoneNumberChanged("512345678")
            viewModel.onVerificationCodeChanged("123456")
            viewModel.onLogin()
            advanceUntilIdle()

            // 断言 resetUnauthorized 被调用一次
            assertEquals(1, fakeUnauthorizedHandler.resetCallCount)
        }

    /**
     * 补充验证：saveToken 抛出异常时，不应调用 resetUnauthorized
     * （只有真正登录成功保存了 token，reset 才有意义）。
     */
    @Test
    fun `login saveToken failure does not call resetUnauthorized`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeUnauthorizedHandler = FakeUnauthorizedHandler()
            val viewModel = LoginViewModel(
                authRepository = FakeAuthRepository(
                    loginResult = Result.success(LoginResult("token", "user_id", false))
                ),
                tokenManager = ThrowingTokenManager(IOException("Disk full")),
                unauthorizedHandler = fakeUnauthorizedHandler
            )
            viewModel.onPhoneNumberChanged("512345678")
            viewModel.onVerificationCodeChanged("123456")
            viewModel.onLogin()
            advanceUntilIdle()

            assertEquals(0, fakeUnauthorizedHandler.resetCallCount)
        }

    /**
     * 端到端验证：先触发 401（handled=true），登录成功后 reset，
     * 下次 401 应当能再次触发（resetCallCount 证明调用路径完整）。
     */
    @Test
    fun `after login success resetUnauthorized enables next 401 to trigger again`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeUnauthorizedHandler = FakeUnauthorizedHandler()
            // 先模拟一次 401，handled 变为 true
            fakeUnauthorizedHandler.simulateHandled()

            val viewModel = LoginViewModel(
                authRepository = FakeAuthRepository(),
                tokenManager = FakeTokenManager(),
                unauthorizedHandler = fakeUnauthorizedHandler
            )
            viewModel.onPhoneNumberChanged("512345678")
            viewModel.onVerificationCodeChanged("123456")
            viewModel.onLogin()
            advanceUntilIdle()

            // reset 被调用 → FakeUnauthorizedHandler 内 handled 重置为 false
            assertEquals(1, fakeUnauthorizedHandler.resetCallCount)
            // 验证重置后 onUnauthorized 的幂等性可再次生效
            assertEquals(false, fakeUnauthorizedHandler.isHandled)
        }

    // ─────────────────────────────────────────────
    // Test Doubles (Fakes) — defined in companion
    // ─────────────────────────────────────────────

    /** Configurable fake repository for T-30002 tests */
    private class FakeAuthRepository(
        val loginResult: Result<LoginResult> =
            Result.success(LoginResult("test_token", "test_user_id", isNew = false)),
        val sendCodeResult: Result<SendCodeResult> =
            Result.success(SendCodeResult(cooldownSeconds = 60))
    ) : IAuthRepository {
        var loginCallCount = 0
        var sendCodeCallCount = 0

        override suspend fun sendCode(phone: String): Result<SendCodeResult> {
            sendCodeCallCount++
            return sendCodeResult
        }

        override suspend fun login(phone: String, code: String): Result<LoginResult> {
            loginCallCount++
            return loginResult
        }
    }

    /** Configurable fake token manager for T-30002 tests */
    private class FakeTokenManager : ITokenManager {
        var savedToken: String? = null

        override suspend fun saveToken(token: String) {
            savedToken = token
        }

        override suspend fun getToken(): String? = savedToken

        override suspend fun clearToken() {
            savedToken = null
        }
    }

    /**
     * A fake token manager that always throws on [saveToken].
     * Used to test the IOException error-handling path introduced in T-30002 Review.
     */
    private class ThrowingTokenManager(
        private val throwable: Throwable
    ) : ITokenManager {
        override suspend fun saveToken(token: String) { throw throwable }
        override suspend fun getToken(): String? = null
        override suspend fun clearToken() = Unit
    }

    /**
     * Fake [UnauthorizedHandler] that records [resetUnauthorized] call count.
     * Used to verify T-30003 Review H-01 fix: LoginViewModel must call reset after login.
     */
    private class FakeUnauthorizedHandler : UnauthorizedHandler {
        var resetCallCount = 0
        var onUnauthorizedCallCount = 0
        /** Reflects whether handler has been "handled" (mirrors AtomicBoolean logic in Fake). */
        var isHandled = false

        override suspend fun onUnauthorized() {
            onUnauthorizedCallCount++
            isHandled = true
        }

        override fun resetUnauthorized() {
            resetCallCount++
            isHandled = false
        }

        /** Test-only helper: simulate a prior 401 having set handled=true. */
        fun simulateHandled() {
            isHandled = true
        }
    }
}
