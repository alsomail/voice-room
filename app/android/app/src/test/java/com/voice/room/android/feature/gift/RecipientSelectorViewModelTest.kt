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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — RecipientSelector ViewModel 逻辑 (T-30029)
 *
 * R29-01: 只显示在麦的用户（由调用方过滤传入）
 * R29-02: 首次渲染默认选主麦（slot=0）
 * R29-03: 点击切换选中项
 * R29-04: 原选中用户下麦后自动切换到主麦
 * R29-05: 全部下麦后 selectedRecipientId=null + canSend=false
 * R29-07: 新用户上麦后 recipients 列表更新
 * Sort-01: 麦位用户按 micIndex 升序排序（slot=0 置首）
 * Sort-02: 已选中用户跨排序保持不变
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RecipientSelectorViewModelTest {

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

    private fun buildViewModel(
        giftRepository: IGiftRepository = FakeGiftRepository(),
        wsClient: FakeWebSocketClient = FakeWebSocketClient(),
    ) = GiftPanelViewModel(giftRepository = giftRepository, wsClient = wsClient)

    /**
     * 创建测试用在麦用户（T-30029 新增 micIndex 字段）
     */
    private fun makeMicUser(
        micIndex: Int,
        userId: String = "u$micIndex",
        nickname: String = "User$micIndex",
    ) = MicUserVO(
        micIndex = micIndex,
        userId = userId,
        nickname = nickname,
        avatarUrl = null,
    )

    // ─── R29-01: 只显示在麦的用户 ─────────────────────────────────────────────

    @Test
    fun `R29-01 only on-mic users shown in recipients`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            // 外部只传在麦的用户（空麦位由调用方过滤，不传入）
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 3, userId = "u3"),
            ))

            assertEquals("Only on-mic users should be shown",
                2, vm.uiState.value.recipients.size)
            assertTrue("host should be in recipients",
                vm.uiState.value.recipients.any { it.userId == "host" })
            assertTrue("u3 should be in recipients",
                vm.uiState.value.recipients.any { it.userId == "u3" })
        }

    // ─── R29-02: 首次渲染默认选主麦（slot=0）─────────────────────────────────

    @Test
    fun `R29-02 first render auto-selects main mic slot=0`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            val recipients = listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 5, userId = "u5"),
            )
            vm.updateRecipients(recipients)

            assertEquals("Should auto-select main mic (slot=0)",
                "host", vm.uiState.value.selectedRecipientId)
        }

    @Test
    fun `R29-02b first render auto-selects slot=0 even if passed out of order`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            // 乱序传入，slot=0 在最后
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 5, userId = "u5"),
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 0, userId = "host"),
            ))

            assertEquals("Should auto-select slot=0 (main mic) even when passed last",
                "host", vm.uiState.value.selectedRecipientId)
        }

    // ─── R29-03: 点击切换选中项 ───────────────────────────────────────────────

    @Test
    fun `R29-03 click recipient switches selection`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
            ))
            assertEquals("Should auto-select host initially",
                "host", vm.uiState.value.selectedRecipientId)

            vm.selectRecipient("u2")

            assertEquals("Should switch to u2 after click",
                "u2", vm.uiState.value.selectedRecipientId)
        }

    // ─── R29-04: 原选中用户下麦后自动切换到主麦 ──────────────────────────────

    @Test
    fun `R29-04 when selected user leaves mic auto switch to main mic`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            // 初始：host + u2 都在麦上，选中 u2
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
            ))
            vm.selectRecipient("u2")
            assertEquals("Pre-condition: u2 selected", "u2",
                vm.uiState.value.selectedRecipientId)

            // u2 下麦，只剩 host
            vm.updateRecipients(listOf(makeMicUser(micIndex = 0, userId = "host")))

            assertEquals("Should auto-switch to main mic when selected user leaves",
                "host", vm.uiState.value.selectedRecipientId)
        }

    @Test
    fun `R29-04b selected user leaves and multiple users remain auto selects slot=0`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            // 选中 u5（slot=5）
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 5, userId = "u5"),
            ))
            vm.selectRecipient("u5")

            // u5 下麦，host 和 u2 剩余
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 0, userId = "host"),
            ))

            assertEquals("Should switch to slot=0 (host) after selected user leaves",
                "host", vm.uiState.value.selectedRecipientId)
        }

    // ─── R29-05: 全部下麦 ─────────────────────────────────────────────────────

    @Test
    fun `R29-05 all leave mic selectedRecipientId becomes null canSend false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val gift = GiftVO("g1", "c1", "Gift", "", 10L, 1, 1)
            val repo = FakeGiftRepository(Result.success(listOf(gift)))
            val wsClient = FakeWebSocketClient()
            val vm = buildViewModel(giftRepository = repo, wsClient = wsClient)
            advanceUntilIdle()

            // 给余额
            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"x",""" +
                    """"payload":{"diamond_balance":100,"delta":100,"reason":"recharge","ref_id":null},""" +
                    """"timestamp":0}"""
            )
            advanceUntilIdle()

            // 有人在麦，选礼物
            vm.updateRecipients(listOf(makeMicUser(micIndex = 0, userId = "host")))
            vm.selectGift("g1")

            assertTrue("Pre-condition: canSend should be true",
                vm.uiState.value.canSend)

            // 全部下麦
            vm.updateRecipients(emptyList())

            assertNull("selectedRecipientId should be null when no one on mic",
                vm.uiState.value.selectedRecipientId)
            assertFalse("canSend should be false when no one on mic",
                vm.uiState.value.canSend)
        }

    @Test
    fun `R29-05b empty recipients shows empty state in uiState`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(emptyList())

            assertTrue("recipients should be empty",
                vm.uiState.value.recipients.isEmpty())
            assertNull("selectedRecipientId should be null",
                vm.uiState.value.selectedRecipientId)
        }

    // ─── R29-07: 新用户上麦后 recipients 列表更新 ─────────────────────────────

    @Test
    fun `R29-07 new user joins mic appears in recipients list`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(listOf(makeMicUser(micIndex = 0, userId = "host")))
            assertEquals("Initially 1 recipient", 1, vm.uiState.value.recipients.size)

            // 新用户上麦（模拟 3s 内触发，实际为立即）
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 3, userId = "newUser"),
            ))

            assertEquals("Should have 2 recipients after new user joins",
                2, vm.uiState.value.recipients.size)
            assertTrue("New user should be in recipients list",
                vm.uiState.value.recipients.any { it.userId == "newUser" })
        }

    @Test
    fun `R29-07b new user join preserves existing selection`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
            ))
            vm.selectRecipient("u2")

            // 新用户上麦，u2 仍在麦
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 4, userId = "newUser"),
            ))

            assertEquals("Selection of u2 should be preserved when new user joins",
                "u2", vm.uiState.value.selectedRecipientId)
        }

    // ─── Sort-01: 按 micIndex 升序排序，slot=0 置首 ──────────────────────────

    @Test
    fun `Sort-01 recipients sorted by micIndex ascending slot=0 first`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            // 乱序传入（slot=5, slot=0, slot=2）
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 5, userId = "u5"),
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
            ))

            val recipients = vm.uiState.value.recipients
            assertEquals("First should be slot=0 (main mic)", "host", recipients[0].userId)
            assertEquals("Second should be slot=2", "u2", recipients[1].userId)
            assertEquals("Third should be slot=5", "u5", recipients[2].userId)
            assertEquals("micIndex[0] should be 0", 0, recipients[0].micIndex)
            assertEquals("micIndex[1] should be 2", 2, recipients[1].micIndex)
            assertEquals("micIndex[2] should be 5", 5, recipients[2].micIndex)
        }

    // ─── Sort-02: 跨排序保持已选 ──────────────────────────────────────────────

    @Test
    fun `Sort-02 sorting does not change explicitly selected recipient`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel()
            advanceUntilIdle()

            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 0, userId = "host"),
                makeMicUser(micIndex = 2, userId = "u2"),
            ))
            vm.selectRecipient("u2")

            // 重新传入乱序（u2 仍在麦），选中状态应该保持
            vm.updateRecipients(listOf(
                makeMicUser(micIndex = 2, userId = "u2"),
                makeMicUser(micIndex = 0, userId = "host"),
            ))

            assertEquals("Explicit selection should be preserved after re-sort",
                "u2", vm.uiState.value.selectedRecipientId)
        }
}
