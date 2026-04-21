package com.voice.room.android.feature.wallet

import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.domain.wallet.TxnsPage
import com.voice.room.android.domain.wallet.WalletTxn
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — WalletViewModel (T-30027)
 *
 * W27-01: 页面初始化时调用 getBalance()，loadingBalance 变为 false
 * W27-02: 余额为 0 时 uiState.balance=0，无负号
 * W27-03: onRechargeClick() 触发 ShowToast("即将上线")
 * W27-04: WS BalanceUpdated 消息 → balance 更新为新值
 * W27-05: refresh() 重新拉取余额（第 2 次调用 getBalance）
 * W27-08: getBalance 返回 401 错误 → 发射 NavigateToLogin 事件
 * Extra-01: getBalance IOException → uiState.error 非 null
 * Extra-02: balance=Long.MAX_VALUE 极值不崩溃
 * Extra-03: CancellationException 被正确 re-throw，不触发 Error 状态
 * Extra-04: WS 非 BalanceUpdated 消息不影响余额
 */
@OptIn(ExperimentalCoroutinesApi::class)
class WalletViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake WalletRepository ────────────────────────────────────────────────

    private class FakeWalletRepository(
        var balanceResult: Result<Long> = Result.success(1000L),
        var txnResult: Result<TxnsPage> = Result.success(
            TxnsPage(
                items = listOf(
                    WalletTxn("t1", 100L, "礼物收入", null, "2026-01-01T00:00:00Z")
                ),
                total = 1,
                page = 1,
            )
        ),
        private val throwCancellation: Boolean = false,
    ) : IWalletRepository {
        var getBalanceCallCount = 0
        var listTxnsCallCount = 0

        override fun walletPreviewLabel(): String = "Fake Wallet"

        override suspend fun getBalance(): Result<Long> {
            getBalanceCallCount++
            if (throwCancellation) throw CancellationException("cancelled")
            return balanceResult
        }

        override suspend fun listTxns(page: Int, size: Int): Result<TxnsPage> {
            listTxnsCallCount++
            return txnResult
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun buildViewModel(
        walletRepository: IWalletRepository = FakeWalletRepository(),
        wsClient: FakeWebSocketClient = FakeWebSocketClient(),
    ): WalletViewModel = WalletViewModel(walletRepository, wsClient)

    // ─── W27-01: init → getBalance() 被调用，loadingBalance 变 false ──────────

    @Test
    fun `W27-01 init triggers getBalance and loadingBalance becomes false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository()
            val vm = buildViewModel(walletRepository = fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertTrue(
                "getBalance should be called at least once on init",
                fakeRepo.getBalanceCallCount >= 1,
            )
            assertEquals(
                "loadingBalance should be false after success",
                false,
                vm.uiState.value.loadingBalance,
            )
            assertEquals(
                "balance should match repository return value",
                1000L,
                vm.uiState.value.balance,
            )
        }

    // ─── W27-02: balance=0 → uiState.balance=0 ───────────────────────────────

    @Test
    fun `W27-02 balance zero is correctly reflected with no negative sign`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(0L))
            val vm = buildViewModel(walletRepository = fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "balance=0 must be exactly 0, not negative",
                0L,
                vm.uiState.value.balance,
            )
            assertTrue(
                "balance must not be negative",
                vm.uiState.value.balance >= 0L,
            )
        }

    // ─── W27-03: onRechargeClick() → ShowToast("即将上线") ───────────────────

    @Test
    fun `W27-03 onRechargeClick emits ShowToast with coming soon message`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()

            val events = mutableListOf<WalletEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            vm.onRechargeClick()
            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "Should emit ShowToast with '即将上线', got: $events",
                events.any { it is WalletEvent.ShowToast && it.message == "即将上线" },
            )
        }

    // ─── W27-04: WS BalanceUpdated → balance 更新 ────────────────────────────

    @Test
    fun `W27-04 WS BalanceUpdated message updates balance to new value`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(wsClient = wsClient)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            // 初始余额应为 1000
            assertEquals(1000L, vm.uiState.value.balance)

            // 模拟 WS BalanceUpdated 消息（正确协议格式 §6.4.1：payload.diamond_balance）
            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"uuid-w2704","payload":{"diamond_balance":2500,"delta":1500,"reason":"recharge"},"timestamp":1720000000000}"""
            )
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "Balance should update to 2500 after WS BalanceUpdated",
                2500L,
                vm.uiState.value.balance,
            )
        }

    // ─── W27-05: refresh() → getBalance 被调用第 2 次 ────────────────────────

    @Test
    fun `W27-05 refresh reloads balance and increments getBalance call count`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository()
            val vm = buildViewModel(walletRepository = fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            val countAfterInit = fakeRepo.getBalanceCallCount

            // 触发 refresh
            vm.refresh()
            advanceUntilIdle()
            job.cancel()

            assertTrue(
                "getBalance should be called again after refresh, " +
                    "before=$countAfterInit, after=${fakeRepo.getBalanceCallCount}",
                fakeRepo.getBalanceCallCount > countAfterInit,
            )
        }

    // ─── W27-08: 401 错误 → NavigateToLogin ──────────────────────────────────

    @Test
    fun `W27-08 getBalance 401 ApiException emits NavigateToLogin event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository(
                balanceResult = Result.failure(ApiException(401, "Unauthorized"))
            )
            val vm = buildViewModel(walletRepository = fakeRepo)

            val events = mutableListOf<WalletEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "Should emit NavigateToLogin on 401, got: $events",
                events.contains(WalletEvent.NavigateToLogin),
            )
        }

    // ─── Extra-01: IOException → uiState.error 非 null ──────────────────────

    @Test
    fun `Extra-01 IOException sets error in uiState`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository(
                balanceResult = Result.failure(IOException("Network unavailable"))
            )
            val vm = buildViewModel(walletRepository = fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertNotNull(
                "uiState.error should be non-null after IOException",
                vm.uiState.value.error,
            )
            assertEquals(false, vm.uiState.value.loadingBalance)
        }

    // ─── Extra-02: Long.MAX_VALUE 极值不崩溃 ─────────────────────────────────

    @Test
    fun `Extra-02 balance Long MAX_VALUE does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(Long.MAX_VALUE))
            val vm = buildViewModel(walletRepository = fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertEquals(Long.MAX_VALUE, vm.uiState.value.balance)
        }

    // ─── Extra-03: CancellationException re-throw ─────────────────────────────

    @Test
    fun `Extra-03 CancellationException does not emit ShowToast and state is not Error`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeWalletRepository(throwCancellation = true)
            val vm = buildViewModel(walletRepository = fakeRepo)

            val events = mutableListOf<WalletEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "CancellationException must not emit ShowToast, got: $events",
                events.none { it is WalletEvent.ShowToast },
            )
            assertNull(
                "uiState.error must be null when CancellationException is thrown",
                vm.uiState.value.error,
            )
        }

    // ─── Extra-04: 非 BalanceUpdated WS 消息不影响余额 ───────────────────────

    @Test
    fun `Extra-04 non-BalanceUpdated WS message does not change balance`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(500L))
            val vm = buildViewModel(walletRepository = fakeRepo, wsClient = wsClient)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            assertEquals(500L, vm.uiState.value.balance)

            // Simulate unrelated WS message
            wsClient.simulateMessage("""{"type":"UserJoined","userId":"u123"}""")
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "Balance should remain 500 for unrelated WS message",
                500L,
                vm.uiState.value.balance,
            )
        }

    // ─── Extra-05: WS BalanceUpdated with invalid JSON silently ignored ───────

    @Test
    fun `Extra-05 malformed WS JSON message silently ignored`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(300L))
            val vm = buildViewModel(walletRepository = fakeRepo, wsClient = wsClient)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            assertEquals(300L, vm.uiState.value.balance)

            // Send malformed JSON
            wsClient.simulateMessage("not-json-at-all{{{")
            advanceUntilIdle()
            job.cancel()

            // Balance unchanged and no crash
            assertEquals(
                "Balance unchanged after malformed JSON",
                300L,
                vm.uiState.value.balance,
            )
        }

    // ─── R1-CRITICAL-1: WS BalanceUpdated 必须读 payload.diamond_balance ──────
    // RED: 当前代码读 "new_balance"（顶层），新测试使用正确协议格式，应 FAIL

    @Test
    fun `R1-CRITICAL-1 WS BalanceUpdated correct protocol format payload_diamond_balance updates balance`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(1000L))
            val vm = buildViewModel(walletRepository = fakeRepo, wsClient = wsClient)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            assertEquals(1000L, vm.uiState.value.balance)

            // 正确协议格式：payload.diamond_balance（§6.4.1）
            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"uuid-001","payload":{"diamond_balance":4800,"delta":-200,"reason":"gift_send"},"timestamp":1720000000000}"""
            )
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "Balance should update to 4800 from payload.diamond_balance (correct protocol format)",
                4800L,
                vm.uiState.value.balance,
            )
        }

    // ─── R1-CRITICAL-1b: 旧格式顶层 new_balance 不应更新余额 ─────────────────
    // 验证修复后旧的错误格式被忽略（diamond_balance 缺失时视为无效消息）

    @Test
    fun `R1-CRITICAL-1b WS BalanceUpdated with old wrong format top-level new_balance is ignored`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val fakeRepo = FakeWalletRepository(balanceResult = Result.success(1000L))
            val vm = buildViewModel(walletRepository = fakeRepo, wsClient = wsClient)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            assertEquals(1000L, vm.uiState.value.balance)

            // 错误格式（顶层 new_balance，无 payload）
            wsClient.simulateMessage("""{"type":"BalanceUpdated","new_balance":9999}""")
            advanceUntilIdle()
            job.cancel()

            assertEquals(
                "Old wrong format (top-level new_balance) should be ignored after fix",
                1000L,
                vm.uiState.value.balance,
            )
        }

    // ─── R1-HIGH-3: refresh() 遇到 401 应发射 NavigateToLogin ─────────────────
    // RED: 当前 refresh() 的 onFailure 仅设置 error 消息，未检测 401，应 FAIL

    @Test
    fun `R1-HIGH-3 refresh with 401 ApiException emits NavigateToLogin event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 初始化时成功，refresh 时返回 401
            var callCount = 0
            val fakeRepo = object : IWalletRepository {
                override fun walletPreviewLabel(): String = "Fake"
                override suspend fun getBalance(): Result<Long> {
                    callCount++
                    return if (callCount == 1) Result.success(500L)
                    else Result.failure(ApiException(401, "Unauthorized"))
                }
                override suspend fun listTxns(page: Int, size: Int): Result<TxnsPage> =
                    Result.success(TxnsPage(emptyList(), 0, 1))
            }
            val vm = buildViewModel(walletRepository = fakeRepo)

            val events = mutableListOf<WalletEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            advanceUntilIdle()
            // 确认初始化成功，无 NavigateToLogin
            assertTrue("No NavigateToLogin on init success", events.none { it == WalletEvent.NavigateToLogin })

            // 触发 refresh，此时返回 401
            vm.refresh()
            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "refresh() with 401 must emit NavigateToLogin, got: $events",
                events.contains(WalletEvent.NavigateToLogin),
            )
            assertEquals(
                "refreshing should be false after 401",
                false,
                vm.uiState.value.refreshing,
            )
        }

    // ─── R1-HIGH-3b: refresh() 非 401 错误不应发射 NavigateToLogin ─────────────

    @Test
    fun `R1-HIGH-3b refresh with non-401 error sets error message not NavigateToLogin`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var callCount = 0
            val fakeRepo = object : IWalletRepository {
                override fun walletPreviewLabel(): String = "Fake"
                override suspend fun getBalance(): Result<Long> {
                    callCount++
                    return if (callCount == 1) Result.success(500L)
                    else Result.failure(IOException("Network error"))
                }
                override suspend fun listTxns(page: Int, size: Int): Result<TxnsPage> =
                    Result.success(TxnsPage(emptyList(), 0, 1))
            }
            val vm = buildViewModel(walletRepository = fakeRepo)

            val events = mutableListOf<WalletEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            advanceUntilIdle()
            vm.refresh()
            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "Non-401 refresh error must NOT emit NavigateToLogin, got: $events",
                events.none { it == WalletEvent.NavigateToLogin },
            )
            assertNotNull(
                "Non-401 refresh error must set uiState.error",
                vm.uiState.value.error,
            )
        }
}
