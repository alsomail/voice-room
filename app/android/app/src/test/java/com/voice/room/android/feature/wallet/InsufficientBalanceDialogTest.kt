package com.voice.room.android.feature.wallet

import com.google.gson.JsonParser
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.gift.MicUserVO
import com.voice.room.android.feature.gift.GiftPanelEvent
import com.voice.room.android.feature.gift.GiftPanelViewModel
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — 余额不足引导弹窗 (T-30032)
 *
 * I32-01: Server 返回 40290 后 uiState.showInsufficientDialog = true
 * I32-02: showInsufficientDialog=true 时 uiState 包含当前余额与所需金额
 * I32-03: dismissInsufficientDialog() → showInsufficientDialog=false，selectedGiftId 不变
 * I32-04: onGoToWallet() → 发射 NavigateToWallet 事件 + showInsufficientDialog=false
 * I32-05: dismissInsufficientDialog() 不清除 selectedGiftId（状态保留逻辑）
 *          注意：TDS I32-05 UI 层"外部点击不关闭"（dismissOnClickOutside=false）
 *          行为需通过 Instrumented UI 测试验证，此处仅覆盖 ViewModel 状态层面
 * I32-06: 差额计算正确（totalPrice - balance）
 * I32-07: showInsufficientDialog 初始值为 false
 * I32-08: 连续两次 40290 后 dismissInsufficientDialog()，状态恢复 false
 * I32-09: 余额为 0 时 isBalanceInsufficient=true，showInsufficientDialog 初始=false
 * I32-10: onGoToWallet() 后 selectedGiftId 被清除（关闭面板效果）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class InsufficientBalanceDialogTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake GiftRepository ──────────────────────────────────────────────────

    private class FakeGiftRepository(
        var listGiftsResult: Result<List<GiftVO>> = Result.success(emptyList()),
    ) : IGiftRepository {
        override fun featuredGiftLabel(): String = "Fake Gift"
        override suspend fun listGifts(locale: String): Result<List<GiftVO>> = listGiftsResult
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun makeGift(
        id: String = "gift-1",
        price: Long = 100L,
    ) = GiftVO(
        id = id,
        code = "code_$id",
        name = "礼物$id",
        iconUrl = "https://cdn.example.com/$id.png",
        price = price,
        sortOrder = 1,
        tier = 2,
    )

    private fun makeMicUser(userId: String = "user-1") =
        MicUserVO(userId = userId, nickname = "User$userId", avatarUrl = null, micIndex = 0)

    /**
     * 构造已就绪 ViewModel（礼物/接收者/余额全部设置完毕），WS 处于 Connected 状态。
     *
     * 正确初始化顺序（参照 SendFlowTest.buildReadyViewModel）：
     * 1. 创建 VM + advanceUntilIdle → loadGifts 完成
     * 2. 设置接收者/礼物/数量
     * 3. WS 推送余额 + advanceUntilIdle
     * 4. simulateConnect() → send() 可写入 sentMessages
     */
    private fun TestScope.buildReadyViewModel(
        balance: Long = 200L,
        giftPrice: Long = 100L,
        wsClient: FakeWebSocketClient = FakeWebSocketClient(),
        giftId: String = "gift-1",
    ): GiftPanelViewModel {
        val gift = makeGift(id = giftId, price = giftPrice)
        val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
        val vm = GiftPanelViewModel(
            giftRepository = repo,
            wsClient = wsClient,
        )
        advanceUntilIdle()

        vm.updateRecipients(listOf(makeMicUser("u1")))
        vm.selectGift(gift.id)
        vm.selectCount(1)

        wsClient.simulateMessage(
            """{"type":"BalanceUpdated","msg_id":"bal-init","payload":{"diamond_balance":$balance,"delta":$balance,"reason":"init"},"timestamp":1720000000000}"""
        )
        advanceUntilIdle()

        // Connected 状态使 send() 可正常写入 sentMessages
        wsClient.simulateConnect()
        return vm
    }

    /** 发送礼物并模拟服务端回复 40290，令 showInsufficientDialog=true */
    private fun TestScope.triggerInsufficientDialog(
        vm: GiftPanelViewModel,
        wsClient: FakeWebSocketClient,
    ) {
        vm.sendGift()
        runCurrent() // 推进到第一个 suspend 点（WS send 完成）

        assertTrue(
            "sentMessages should have at least one message",
            wsClient.sentMessages.isNotEmpty(),
        )
        val msgId = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
            .get("msg_id").asString
        wsClient.simulateMessage(
            """{"type":"SendGiftResult","msg_id":"$msgId","code":40290}"""
        )
        advanceUntilIdle()
    }

    // ─── I32-01 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-01 server returns 40290 - showInsufficientDialog becomes true`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(balance = 200L, giftPrice = 100L, wsClient = wsClient)

            assertTrue("前置：canSend 应为 true", vm.uiState.value.canSend)

            triggerInsufficientDialog(vm, wsClient)

            assertTrue(
                "showInsufficientDialog should be true after server returns 40290",
                vm.uiState.value.showInsufficientDialog,
            )
        }

    // ─── I32-02 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-02 when showInsufficientDialog is true - state exposes balance and required amount`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(balance = 200L, giftPrice = 100L, wsClient = wsClient)

            triggerInsufficientDialog(vm, wsClient)

            val state = vm.uiState.value
            assertTrue("showInsufficientDialog should be true", state.showInsufficientDialog)
            // Dialog 可从 state 读取展示所需数据
            assertEquals("balance (current diamonds)", 200L, state.balance)
            assertEquals("totalPrice (required diamonds)", 100L, state.totalPrice)
        }

    // ─── I32-03 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-03 dismissInsufficientDialog clears showInsufficientDialog but keeps selectedGiftId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            triggerInsufficientDialog(vm, wsClient)

            // 前置确认
            assertTrue("dialog should be showing", vm.uiState.value.showInsufficientDialog)
            assertNotNull("selectedGiftId should be set", vm.uiState.value.selectedGiftId)
            val giftIdBeforeDismiss = vm.uiState.value.selectedGiftId

            // 点击"取消"
            vm.dismissInsufficientDialog()

            assertFalse(
                "showInsufficientDialog should be false after dismissInsufficientDialog",
                vm.uiState.value.showInsufficientDialog,
            )
            assertEquals(
                "selectedGiftId must remain unchanged (gift panel retains selection)",
                giftIdBeforeDismiss,
                vm.uiState.value.selectedGiftId,
            )
        }

    // ─── I32-04 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-04 onGoToWallet emits NavigateToWallet event and sets showInsufficientDialog=false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            triggerInsufficientDialog(vm, wsClient)

            assertTrue("dialog should be visible before onGoToWallet", vm.uiState.value.showInsufficientDialog)

            // 点击"去充值"
            vm.onGoToWallet()
            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "NavigateToWallet event must be emitted, got: $events",
                events.any { it is GiftPanelEvent.NavigateToWallet },
            )
            assertFalse(
                "showInsufficientDialog should be false after onGoToWallet",
                vm.uiState.value.showInsufficientDialog,
            )
        }

    // ─── I32-05 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-05 dismissInsufficientDialog preserves selectedGiftId - panel retains selection state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            triggerInsufficientDialog(vm, wsClient)

            val selectedGiftIdBeforeDismiss = vm.uiState.value.selectedGiftId
            assertNotNull("selectedGiftId should not be null before dismiss", selectedGiftIdBeforeDismiss)

            vm.dismissInsufficientDialog()

            assertEquals(
                "selectedGiftId must be preserved after dismissInsufficientDialog " +
                    "(ViewModel state retention; UI层 dismissOnClickOutside=false 行为需 Instrumented 测试验证)",
                selectedGiftIdBeforeDismiss,
                vm.uiState.value.selectedGiftId,
            )
            assertFalse(
                "showInsufficientDialog must be false",
                vm.uiState.value.showInsufficientDialog,
            )
        }

    // ─── I32-06 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-06 deficit amount equals totalPrice minus balance`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val balance = 30L
            val giftPrice = 100L
            val vm = buildReadyViewModel(balance = balance, giftPrice = giftPrice, wsClient = wsClient)

            val state = vm.uiState.value
            assertEquals("totalPrice should equal giftPrice", giftPrice, state.totalPrice)
            assertEquals("balance should be $balance", balance, state.balance)
            // Dialog 展示差额 = totalPrice - balance = 70
            assertEquals(
                "deficit = totalPrice - balance should be 70",
                70L,
                state.totalPrice - state.balance,
            )
        }

    // ─── I32-07 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-07 showInsufficientDialog defaults to false on init`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = GiftPanelViewModel(
                giftRepository = FakeGiftRepository(),
                wsClient = FakeWebSocketClient(),
            )
            advanceUntilIdle()

            assertFalse(
                "showInsufficientDialog should be false by default",
                vm.uiState.value.showInsufficientDialog,
            )
        }

    // ─── I32-08 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-08 dialog can be shown and dismissed multiple times`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            // 第一次循环
            triggerInsufficientDialog(vm, wsClient)
            assertTrue("1st: dialog should show after 40290", vm.uiState.value.showInsufficientDialog)

            vm.dismissInsufficientDialog()
            assertFalse("1st: dialog should be dismissed", vm.uiState.value.showInsufficientDialog)

            // 第二次循环（重新 connect 使 send() 再次可用）
            wsClient.simulateConnect()
            triggerInsufficientDialog(vm, wsClient)
            assertTrue("2nd: dialog should show again after 40290", vm.uiState.value.showInsufficientDialog)

            vm.dismissInsufficientDialog()
            assertFalse("2nd: dialog should be dismissed again", vm.uiState.value.showInsufficientDialog)
        }

    // ─── I32-09 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-09 balance zero - isBalanceInsufficient true and showInsufficientDialog initially false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gift = makeGift(price = 50L)
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
            val vm = GiftPanelViewModel(
                giftRepository = repo,
                wsClient = FakeWebSocketClient(),
            )
            advanceUntilIdle()

            vm.updateRecipients(listOf(makeMicUser("u1")))
            vm.selectGift(gift.id)
            vm.selectCount(1)

            val state = vm.uiState.value
            assertEquals("balance should be 0 (default)", 0L, state.balance)
            assertEquals("totalPrice should be 50", 50L, state.totalPrice)
            assertTrue("isBalanceInsufficient should be true when balance=0 < totalPrice=50",
                state.isBalanceInsufficient)
            assertFalse("showInsufficientDialog is initially false even when balance insufficient",
                state.showInsufficientDialog)
        }

    // ─── I32-10 ───────────────────────────────────────────────────────────────

    @Test
    fun `I32-10 onGoToWallet clears selectedGiftId - gift panel is dismissed`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            triggerInsufficientDialog(vm, wsClient)

            assertNotNull("selectedGiftId should be set before onGoToWallet",
                vm.uiState.value.selectedGiftId)

            vm.onGoToWallet()
            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "NavigateToWallet event must be emitted, got: $events",
                events.any { it is GiftPanelEvent.NavigateToWallet },
            )
            assertNull(
                "selectedGiftId should be null after onGoToWallet (gift panel closed)",
                vm.uiState.value.selectedGiftId,
            )
        }
}
