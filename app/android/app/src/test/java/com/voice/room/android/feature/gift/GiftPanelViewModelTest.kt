package com.voice.room.android.feature.gift

import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.gift.MicUserVO
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — GiftPanelViewModel (T-30028)
 *
 * G28-02: 列表加载后 gifts.size >= 6
 * G28-03: selectGift(id) → selectedGiftId = id, 金色边框由 UI 响应
 * G28-04: 独角兽(price=66) × count=520 → totalPrice=34320
 * G28-05: balance 不足 → canSend=false
 * G28-06: dismiss() → selectedGiftId=null（外部点击 / × / 返回键清除选中态）
 * G28-07: WS BalanceUpdated → balance 更新
 * G28-08: recipients 为空 → canSend=false，无论礼物余额是否满足
 * G28-09: 网络失败 → error 非 null，loading=false
 * G28-10: loadGifts(locale="ar") → repository 被以 "ar" 调用
 *
 * Extra-01: 初始 loading=true → loadGifts 成功后 loading=false
 * Extra-02: selectedCount 默认为 1，selectCount 更新档位
 * Extra-03: canSend=true 当条件全满足（礼物/接收者/余额充足）
 * Extra-04: WS 非 BalanceUpdated 消息不影响余额
 * Extra-05: retryLoad() 重新拉取列表
 * Extra-06: selectRecipient 更新 selectedRecipientId
 * Extra-07: Hot Tab 仅展示 tier∈[2,3] 礼物
 * Extra-08: All Tab 展示全部礼物
 * Extra-09: balance 极值 Long.MAX_VALUE 不崩溃
 * Extra-10: updateRecipients 更新 recipients，默认选中第一个
 */
@OptIn(ExperimentalCoroutinesApi::class)
class GiftPanelViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake GiftRepository ──────────────────────────────────────────────────

    private class FakeGiftRepository(
        var listGiftsResult: Result<List<GiftVO>> = Result.success(emptyList()),
    ) : IGiftRepository {
        var listGiftsCallCount = 0
        var lastLocale: String? = null

        override fun featuredGiftLabel(): String = "Fake Gift"

        override suspend fun listGifts(locale: String): Result<List<GiftVO>> {
            listGiftsCallCount++
            lastLocale = locale
            return listGiftsResult
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun buildViewModel(
        giftRepository: IGiftRepository = FakeGiftRepository(),
        wsClient: FakeWebSocketClient = FakeWebSocketClient(),
    ) = GiftPanelViewModel(
        giftRepository = giftRepository,
        wsClient = wsClient,
    )

    /** 生成指定数量的测试礼物（tier=2 Hot 礼物） */
    private fun makeGifts(count: Int, tier: Int = 2, price: Long = 10L): List<GiftVO> =
        (1..count).map { i ->
            GiftVO(
                id = "gift-$i",
                code = "code_$i",
                name = "礼物$i",
                iconUrl = "https://cdn.example.com/gift$i.png",
                price = price,
                sortOrder = i,
                tier = tier,
            )
        }

    // ─── Tests ────────────────────────────────────────────────────────────────

    // --- G28-02 ---

    @Test
    fun `G28-02 list loaded - gifts size at least 6`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val repo = FakeGiftRepository(listGiftsResult = Result.success(makeGifts(8)))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            val state = vm.uiState.value
            assertTrue("gifts.size should be >= 6, was ${state.gifts.size}", state.gifts.size >= 6)
            assertFalse("loading should be false after success", state.loading)
            assertNull("error should be null on success", state.error)
        }

    // --- G28-03 ---

    @Test
    fun `G28-03 selectGift updates selectedGiftId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gifts = makeGifts(3)
            val repo = FakeGiftRepository(listGiftsResult = Result.success(gifts))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.selectGift(gifts[1].id)

            assertEquals("gift-2", vm.uiState.value.selectedGiftId)
        }

    // --- G28-04 ---

    @Test
    fun `G28-04 unicorn price=66 times count=520 equals totalPrice=34320`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val unicorn = GiftVO(
                id = "unicorn-1",
                code = "unicorn_01",
                name = "独角兽",
                iconUrl = "https://cdn.example.com/unicorn.png",
                price = 66L,
                sortOrder = 1,
                tier = 2,
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(unicorn)))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.selectGift("unicorn-1")
            vm.selectCount(520)

            assertEquals(34320L, vm.uiState.value.totalPrice)
        }

    // --- G28-05 ---

    @Test
    fun `G28-05 canSend false when balance insufficient`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val unicorn = GiftVO(
                id = "g1",
                code = "unicorn_01",
                name = "独角兽",
                iconUrl = "",
                price = 66L,
                sortOrder = 1,
                tier = 2,
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(unicorn)))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.updateRecipients(listOf(MicUserVO("u1", "Alice", null)))
            vm.selectRecipient("u1")
            vm.selectGift("g1")
            vm.selectCount(520) // totalPrice = 66 * 520 = 34320, balance = 0

            assertFalse("canSend should be false when balance=0 < totalPrice=34320",
                vm.uiState.value.canSend)
            assertTrue("isBalanceInsufficient should be true",
                vm.uiState.value.isBalanceInsufficient)
        }

    // --- G28-06 ---

    @Test
    fun `G28-06 dismiss clears selectedGiftId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gifts = makeGifts(1)
            val repo = FakeGiftRepository(listGiftsResult = Result.success(gifts))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.selectGift(gifts[0].id)
            assertNotNull("selectedGiftId should be set", vm.uiState.value.selectedGiftId)

            vm.dismiss()

            assertNull("selectedGiftId should be null after dismiss",
                vm.uiState.value.selectedGiftId)
        }

    // --- G28-07 ---

    @Test
    fun `G28-07 BalanceUpdated WS message updates balance`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(wsClient = wsClient)
            advanceUntilIdle()

            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"uuid-001",""" +
                    """"payload":{"diamond_balance":4800,"delta":-520,"reason":"gift_send","ref_id":null},""" +
                    """"timestamp":1720000000000}"""
            )
            advanceUntilIdle()

            assertEquals("balance should be updated to 4800", 4800L, vm.uiState.value.balance)
        }

    // --- G28-08 ---

    @Test
    fun `G28-08 canSend false when recipients empty`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gift = GiftVO(
                id = "g1",
                code = "c1",
                name = "Gift",
                iconUrl = "",
                price = 10L,
                sortOrder = 1,
                tier = 1,
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.selectGift("g1")
            // recipients is empty by default

            assertFalse("canSend should be false when recipients is empty",
                vm.uiState.value.canSend)
        }

    // --- G28-09 ---

    @Test
    fun `G28-09 network failure sets error and loading=false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val repo = FakeGiftRepository(
                listGiftsResult = Result.failure(IOException("Network error"))
            )
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            val state = vm.uiState.value
            assertNotNull("error should be non-null on failure", state.error)
            assertFalse("loading should be false after failure", state.loading)
            assertTrue("gifts should be empty on failure", state.gifts.isEmpty())
        }

    // --- G28-10 ---

    @Test
    fun `G28-10 loadGifts with locale ar passes locale to repository`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val repo = FakeGiftRepository(listGiftsResult = Result.success(emptyList()))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.loadGifts(locale = "ar")
            advanceUntilIdle()

            assertEquals("locale should be 'ar'", "ar", repo.lastLocale)
        }

    // --- Extra-01 ---

    @Test
    fun `Extra-01 loading becomes false after loadGifts succeeds`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val repo = FakeGiftRepository(listGiftsResult = Result.success(makeGifts(3)))
            val vm = buildViewModel(giftRepository = repo)

            advanceUntilIdle()
            assertFalse("loading should be false after loadGifts completes",
                vm.uiState.value.loading)
        }

    // --- Extra-02 ---

    @Test
    fun `Extra-02 selectedCount defaults to 1 and selectCount updates it`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            assertEquals("default selectedCount should be 1", 1, vm.uiState.value.selectedCount)

            vm.selectCount(66)
            assertEquals("selectedCount should be 66 after selectCount(66)",
                66, vm.uiState.value.selectedCount)
        }

    // --- Extra-03 ---

    @Test
    fun `Extra-03 canSend true when all conditions satisfied`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gift = GiftVO(
                id = "g1",
                code = "c1",
                name = "Gift",
                iconUrl = "",
                price = 10L,
                sortOrder = 1,
                tier = 1,
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(giftRepository = repo, wsClient = wsClient)
            advanceUntilIdle()

            // Set balance via WS
            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"u1",""" +
                    """"payload":{"diamond_balance":100,"delta":100,"reason":"recharge","ref_id":null},""" +
                    """"timestamp":1720000000000}"""
            )
            advanceUntilIdle()

            vm.updateRecipients(listOf(MicUserVO("u1", "Alice", null)))
            vm.selectRecipient("u1")
            vm.selectGift("g1")
            vm.selectCount(1) // totalPrice=10, balance=100 → sufficient

            assertTrue("canSend should be true when all conditions met",
                vm.uiState.value.canSend)
        }

    // --- Extra-04 ---

    @Test
    fun `Extra-04 non-BalanceUpdated WS message does not affect balance`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(wsClient = wsClient)
            advanceUntilIdle()

            val initialBalance = vm.uiState.value.balance
            wsClient.simulateMessage("""{"type":"MessageReceived","msg_id":"x","payload":{}}""")
            advanceUntilIdle()

            assertEquals("balance should not change on unrelated WS message",
                initialBalance, vm.uiState.value.balance)
        }

    // --- Extra-05 ---

    @Test
    fun `Extra-05 retryLoad reloads gifts after failure`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val repo = FakeGiftRepository(
                listGiftsResult = Result.failure(IOException("Network error"))
            )
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            assertNotNull("error should be set initially", vm.uiState.value.error)

            // Fix the repo to succeed
            repo.listGiftsResult = Result.success(makeGifts(6))
            vm.retryLoad()
            advanceUntilIdle()

            assertNull("error should be cleared after retry success", vm.uiState.value.error)
            assertEquals("gifts should be loaded after retry", 6, vm.uiState.value.gifts.size)
        }

    // --- Extra-06 ---

    @Test
    fun `Extra-06 selectRecipient updates selectedRecipientId`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(listOf(
                MicUserVO("u1", "Alice", null),
                MicUserVO("u2", "Bob", null),
            ))
            vm.selectRecipient("u2")

            assertEquals("selectedRecipientId should be u2", "u2",
                vm.uiState.value.selectedRecipientId)
        }

    // --- Extra-07 ---

    @Test
    fun `Extra-07 hot tab displayGifts only shows tier 2 and 3 gifts`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gifts = listOf(
                GiftVO("g1", "c1", "普通礼物", "", 5L, 1, tier = 1),
                GiftVO("g2", "c2", "热门礼物", "", 20L, 2, tier = 2),
                GiftVO("g3", "c3", "精选礼物", "", 50L, 3, tier = 3),
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(gifts))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            // Active tab is Hot by default
            assertEquals("Hot tab should have 2 gifts (tier 2 and 3)",
                2, vm.uiState.value.displayGifts.size)
            assertTrue("Hot tab gifts should not contain tier-1 gift",
                vm.uiState.value.displayGifts.none { it.id == "g1" })
        }

    // --- Extra-08 ---

    @Test
    fun `Extra-08 all tab displayGifts shows all gifts`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gifts = listOf(
                GiftVO("g1", "c1", "普通", "", 5L, 1, tier = 1),
                GiftVO("g2", "c2", "热门", "", 20L, 2, tier = 2),
                GiftVO("g3", "c3", "精选", "", 50L, 3, tier = 3),
            )
            val repo = FakeGiftRepository(listGiftsResult = Result.success(gifts))
            val vm = buildViewModel(giftRepository = repo)
            advanceUntilIdle()

            vm.selectTab(GiftTab.All)

            assertEquals("All tab should show all 3 gifts", 3, vm.uiState.value.displayGifts.size)
        }

    // --- Extra-09 ---

    @Test
    fun `Extra-09 balance Long MAX_VALUE does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(wsClient = wsClient)
            advanceUntilIdle()

            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"u1",""" +
                    """"payload":{"diamond_balance":${Long.MAX_VALUE},"delta":0,"reason":"admin_adjust","ref_id":null},""" +
                    """"timestamp":1720000000000}"""
            )
            advanceUntilIdle()

            assertEquals(Long.MAX_VALUE, vm.uiState.value.balance)
        }

    // --- Extra-10 ---

    @Test
    fun `Extra-10 updateRecipients updates recipients and auto selects first`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            val recipients = listOf(
                MicUserVO("u1", "Alice", null),
                MicUserVO("u2", "Bob", null),
            )
            vm.updateRecipients(recipients)

            val state = vm.uiState.value
            assertEquals("recipients should be updated", 2, state.recipients.size)
            assertEquals("first recipient should be auto-selected", "u1",
                state.selectedRecipientId)
        }
}
