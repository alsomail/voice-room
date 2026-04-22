package com.voice.room.android.feature.gift

import com.google.gson.JsonParser
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.core.ws.event.GiftReceivedEvent
import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.gift.MicUserVO
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.util.UUID

/**
 * TDD 单元测试 — SendGift 发送流程 (T-30030)
 *
 * S30-01: 点击送出 → WS 消息 type=SendGift, msg_id 为合法 UUID
 * S30-02: 发送中按钮 disabled（canSend=false）且 sending=true
 * S30-03: 收到 SendGiftResult code=0 → sending=false，toast "赠送成功"
 * S30-04: 5s 未收到回复 → toast "请求超时，请重试"，sending=false
 * S30-05: code=40290 → 触发 ShowInsufficientDialog 事件
 * S30-06: code=40403 → toast，面板保留（无 DismissPanel 事件）
 * S30-07: ComboAggregator 同礼物同接收者 3s 内 5 次 → count=5
 * S30-08: ComboAggregator 切换礼物后新建 msg_id
 * S30-09: ComboAggregator 每次新 combo msg_id 唯一
 *
 * 额外测试：
 * S30-10: WS JSON 含正确的 gift_id / receiver_id / count / room_id 字段
 * S30-11: 余额不足时 canSend=false，sendGift() 为空操作
 * S30-12: sending=true 期间再次调用 sendGift() 为空操作（幂等保护）
 * S30-13: code=40402 → toast 包含"下架"
 * S30-14: code=40400 → toast + DismissPanel 事件
 * S30-15: ComboAggregator 超出 3s 窗口后重置 msg_id
 */
@OptIn(ExperimentalCoroutinesApi::class)
class SendFlowTest {

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
        price: Long = 10L,
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
     * 构建一个"已就绪可送礼"的 ViewModel。
     *
     * **必须在 TestScope 内调用**（如 runTest {} 块中），因为需要调用 [advanceUntilIdle]。
     *
     * 正确初始化顺序：
     * 1. 创建 VM（触发 loadGifts 协程）
     * 2. [advanceUntilIdle] → loadGifts 完成，gifts 已加载
     * 3. 选中礼物（此时 gifts 非空，selectGift 不再是空操作）
     * 4. 模拟 WS BalanceUpdated 设置余额
     * 5. [advanceUntilIdle] → 余额更新处理完毕
     * 6. [FakeWebSocketClient.simulateConnect] → 恢复 Connected 状态，send() 可工作
     */
    @OptIn(ExperimentalCoroutinesApi::class)
    private fun TestScope.buildReadyViewModel(
        wsClient: FakeWebSocketClient = FakeWebSocketClient(),
        giftId: String = "gift-1",
        recipientId: String = "user-1",
        count: Int = 1,
        balance: Long = 100L,
        roomId: String = "room-123",
    ): GiftPanelViewModel {
        val gift = makeGift(id = giftId, price = 10L)
        val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
        val vm = GiftPanelViewModel(
            giftRepository = repo,
            wsClient = wsClient,
            roomId = roomId,
        )

        // 等待 loadGifts 完成，此后 gifts 列表非空
        advanceUntilIdle()

        // 选中礼物和接收者（gifts 已加载，selectGift 有效）
        vm.updateRecipients(listOf(makeMicUser(recipientId)))
        vm.selectGift(giftId)
        vm.selectCount(count)

        // 模拟 WS 余额推送
        wsClient.simulateMessage(
            """{"type":"BalanceUpdated","msg_id":"bal-uuid","payload":{"diamond_balance":$balance,"delta":$balance,"reason":"recharge","ref_id":null},"timestamp":1720000000000}"""
        )
        // 处理余额更新（BalanceUpdated → balance = $balance）
        advanceUntilIdle()

        // 恢复 Connected 状态，使后续 wsClient.send() 能写入 sentMessages
        wsClient.simulateConnect()

        return vm
    }

    // ─── S30-01 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-01 sendGift sends WS message with type=SendGift and valid UUID msg_id`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            assertTrue("前置：canSend 应为 true", vm.uiState.value.canSend)

            vm.sendGift()
            runCurrent()

            assertTrue(
                "至少有一条 WS 消息发送",
                wsClient.sentMessages.isNotEmpty()
            )
            val json = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
            assertEquals("type 应为 SendGift", "SendGift", json.get("type").asString)

            val msgId = json.get("msg_id").asString
            assertNotNull("msg_id 不应为 null", msgId)
            // 验证是合法 UUID（不抛异常）
            try {
                UUID.fromString(msgId)
            } catch (e: IllegalArgumentException) {
                throw AssertionError("msg_id 不是合法 UUID: $msgId", e)
            }
        }

    // ─── S30-02 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-02 sending=true and canSend=false immediately after sendGift`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            assertTrue("前置条件：canSend 应为 true", vm.uiState.value.canSend)

            vm.sendGift()
            runCurrent() // 执行到第一个 suspend 点

            assertTrue("发送中 sending 应为 true", vm.uiState.value.sending)
            assertFalse("发送中 canSend 应为 false（含 !sending 判断）", vm.uiState.value.canSend)
        }

    // ─── S30-03 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-03 SendGiftResult code=0 restores sending=false and emits toast success`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent() // 执行到 suspend 等待结果处

            // 获取发送的 msg_id
            assertTrue("应有 WS 消息发送", wsClient.sentMessages.isNotEmpty())
            val sentJson = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
            val msgId = sentJson.get("msg_id").asString

            // 模拟 Server 回复 code=0
            wsClient.simulateMessage(
                """{"type":"SendGiftResult","msg_id":"$msgId","code":0,"payload":{}}"""
            )
            advanceUntilIdle()

            assertFalse("code=0 后 sending 应为 false", vm.uiState.value.sending)
            val toastEvent = events.filterIsInstance<GiftPanelEvent.ShowToast>().firstOrNull()
            assertNotNull("应收到 ShowToast 事件", toastEvent)
            assertTrue("Toast 应包含'赠送成功'", toastEvent!!.message.contains("赠送成功"))

            collectJob.cancel()
        }

    // ─── S30-04 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-04 5s timeout triggers timeout toast and restores sending=false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent() // 执行到 suspend 等待结果处

            assertTrue("等待期间 sending 应为 true", vm.uiState.value.sending)

            // 推进虚拟时间 5001ms → 触发 withTimeoutOrNull(5000) 超时
            advanceTimeBy(5001L)
            advanceUntilIdle()

            assertFalse("超时后 sending 应为 false", vm.uiState.value.sending)
            val toastEvent = events.filterIsInstance<GiftPanelEvent.ShowToast>().firstOrNull()
            assertNotNull("超时后应收到 ShowToast 事件", toastEvent)
            assertTrue("Toast 应包含超时提示", toastEvent!!.message.contains("超时"))

            collectJob.cancel()
        }

    // ─── S30-05 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-05 SendGiftResult code=40290 emits ShowInsufficientDialog`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent()

            val msgId = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
                .get("msg_id").asString
            wsClient.simulateMessage(
                """{"type":"SendGiftResult","msg_id":"$msgId","code":40290,"payload":{}}"""
            )
            advanceUntilIdle()

            assertTrue(
                "code=40290 应触发 ShowInsufficientDialog",
                events.any { it is GiftPanelEvent.ShowInsufficientDialog }
            )
            collectJob.cancel()
        }

    // ─── S30-06 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-06 SendGiftResult code=40403 toasts and keeps panel open`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent()

            val msgId = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
                .get("msg_id").asString
            wsClient.simulateMessage(
                """{"type":"SendGiftResult","msg_id":"$msgId","code":40403,"payload":{}}"""
            )
            advanceUntilIdle()

            val toastEvent = events.filterIsInstance<GiftPanelEvent.ShowToast>().firstOrNull()
            assertNotNull("code=40403 应收到 Toast", toastEvent)
            // 不应有 DismissPanel 事件（面板保留）
            assertFalse(
                "code=40403 不应触发 DismissPanel",
                events.any { it is GiftPanelEvent.DismissPanel }
            )

            collectJob.cancel()
        }

    // ─── S30-07 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-07 ComboAggregator same gift and recipient 5 presses within 3s yields count=5`() {
        var fakeNow = 0L
        val aggregator = ComboAggregator(windowMs = 3000L, timeProvider = { fakeNow })

        fakeNow = 0L
        aggregator.press("gift-1", "user-1")
        fakeNow = 500L
        aggregator.press("gift-1", "user-1")
        fakeNow = 1000L
        aggregator.press("gift-1", "user-1")
        fakeNow = 1500L
        aggregator.press("gift-1", "user-1")
        fakeNow = 2000L
        val combo = aggregator.press("gift-1", "user-1")

        assertEquals("3s 内 5 次点击 count 应为 5", 5, combo.count)
    }

    // ─── S30-08 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-08 ComboAggregator switching gift creates new combo with different msg_id`() {
        var fakeNow = 0L
        val aggregator = ComboAggregator(windowMs = 3000L, timeProvider = { fakeNow })

        val combo1 = aggregator.press("gift-1", "user-1")
        fakeNow = 100L
        val combo2 = aggregator.press("gift-2", "user-1") // 切换礼物

        assertNotEquals("切换礼物后应生成新 msg_id", combo1.msgId, combo2.msgId)
        assertEquals("新 combo 的 count 应重置为 1", 1, combo2.count)
    }

    @Test
    fun `S30-08b ComboAggregator switching recipient creates new combo with different msg_id`() {
        var fakeNow = 0L
        val aggregator = ComboAggregator(windowMs = 3000L, timeProvider = { fakeNow })

        val combo1 = aggregator.press("gift-1", "user-1")
        fakeNow = 100L
        val combo2 = aggregator.press("gift-1", "user-2") // 切换接收者

        assertNotEquals("切换接收者后应生成新 msg_id", combo1.msgId, combo2.msgId)
        assertEquals("新 combo 的 count 应重置为 1", 1, combo2.count)
    }

    // ─── S30-09 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-09 each new combo has unique msg_id`() {
        val aggregator = ComboAggregator(windowMs = 3000L)

        // 每次 flush 后新建 combo，均应生成不同 msgId
        val msgIds = (1..50).map { _ ->
            aggregator.flush()
            aggregator.press("gift-1", "user-1").msgId
        }

        val distinct = msgIds.distinct()
        assertEquals("每次新 combo 的 msg_id 均应唯一，共 50 个", 50, distinct.size)
        // 每个都是合法 UUID
        distinct.forEach { id ->
            try {
                UUID.fromString(id)
            } catch (e: IllegalArgumentException) {
                throw AssertionError("msg_id 不是合法 UUID: $id", e)
            }
        }
    }

    // ─── S30-10 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-10 WS JSON contains correct gift_id receiver_id count fields`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(
                wsClient = wsClient,
                giftId = "gift-42",
                recipientId = "user-99",
                count = 10,
                roomId = "room-xyz",
            )

            vm.sendGift()
            runCurrent()

            assertTrue("应有 WS 消息发送", wsClient.sentMessages.isNotEmpty())
            val json = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
            val payload = json.getAsJsonObject("payload")
            assertNotNull("payload 不应为 null", payload)
            assertEquals("gift_id 应匹配", "gift-42", payload.get("gift_id").asString)
            assertEquals("receiver_id 应匹配", "user-99", payload.get("receiver_id").asString)
            assertEquals("count 应为 10", 10, payload.get("count").asInt)
            assertEquals("room_id 应匹配", "room-xyz", payload.get("room_id").asString)
        }

    // ─── S30-11 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-11 sendGift is no-op when balance insufficient`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val gift = makeGift(price = 100L)
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
            val vm = GiftPanelViewModel(giftRepository = repo, wsClient = wsClient, roomId = "r1")

            // 等待 loadGifts 完成
            advanceUntilIdle()

            // 选中礼物（price=100），不设置余额（balance=0，默认）
            vm.updateRecipients(listOf(makeMicUser()))
            vm.selectGift("gift-1")
            wsClient.simulateConnect()

            assertFalse("前置：余额不足 canSend 应为 false", vm.uiState.value.canSend)

            vm.sendGift()
            runCurrent()

            assertTrue("余额不足时不应发送 WS 消息", wsClient.sentMessages.isEmpty())
            assertFalse("余额不足时 sending 不应变为 true", vm.uiState.value.sending)
        }

    // ─── S30-12 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-12 second sendGift is no-op while first is sending`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            vm.sendGift()
            runCurrent() // 执行到 suspend 点（sending=true）

            val messageCountBeforeSecond = wsClient.sentMessages.size
            vm.sendGift() // 第二次调用（sending=true，canSend=false）
            runCurrent()

            assertEquals(
                "第二次 sendGift 不应再发 WS 消息",
                messageCountBeforeSecond,
                wsClient.sentMessages.size
            )
        }

    // ─── S30-13 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-13 SendGiftResult code=40402 shows gift unavailable toast`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent()

            val msgId = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
                .get("msg_id").asString
            wsClient.simulateMessage(
                """{"type":"SendGiftResult","msg_id":"$msgId","code":40402,"payload":{}}"""
            )
            advanceUntilIdle()

            val toast = events.filterIsInstance<GiftPanelEvent.ShowToast>().firstOrNull()
            assertNotNull("code=40402 应有 Toast", toast)
            assertTrue("Toast 应包含下架提示", toast!!.message.contains("下架"))

            collectJob.cancel()
        }

    // ─── S30-14 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-14 SendGiftResult code=40400 shows toast and emits DismissPanel`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val wsClient = FakeWebSocketClient()
            val vm = buildReadyViewModel(wsClient = wsClient)

            val events = mutableListOf<GiftPanelEvent>()
            val collectJob = launch { vm.events.collect { events.add(it) } }

            vm.sendGift()
            runCurrent()

            val msgId = JsonParser.parseString(wsClient.sentMessages.last()).asJsonObject
                .get("msg_id").asString
            wsClient.simulateMessage(
                """{"type":"SendGiftResult","msg_id":"$msgId","code":40400,"payload":{}}"""
            )
            advanceUntilIdle()

            assertTrue(
                "code=40400 应触发 DismissPanel 事件",
                events.any { it is GiftPanelEvent.DismissPanel }
            )

            collectJob.cancel()
        }

    // ─── S30-15 ───────────────────────────────────────────────────────────────

    @Test
    fun `S30-15 ComboAggregator resets after window expires`() {
        var fakeNow = 0L
        val aggregator = ComboAggregator(windowMs = 3000L, timeProvider = { fakeNow })

        val combo1 = aggregator.press("gift-1", "user-1")
        fakeNow = 3001L // 超出 3s 窗口
        val combo2 = aggregator.press("gift-1", "user-1")

        assertNotEquals("超出时间窗口后应生成新 msg_id", combo1.msgId, combo2.msgId)
        assertEquals("超出时间窗口后 count 应重置为 1", 1, combo2.count)
    }

    // ─── HIGH-1：GiftReceivedEvent 含 receiverAvatar 字段 ─────────────────────

    /**
     * HIGH-1（Review R1）：协议 §6.4.3 的 receiver 对象包含 avatar 字段，
     * GiftReceivedEvent 必须有对应的 receiverAvatar: String? 字段。
     *
     * RED 阶段：GiftReceivedEvent 目前缺少 receiverAvatar，编译即失败。
     */
    @Test
    fun `HIGH-1 GiftReceivedEvent has receiverAvatar field matching protocol section 6_4_3`() {
        // 构造时传入 receiverAvatar=null（可 null 以兼容旧协议）
        val eventWithNullAvatar = GiftReceivedEvent(
            msgId = "msg-high1",
            giftRecordId = "record-1",
            senderUserId = "sender-1",
            senderNickname = "Alice",
            senderAvatar = "https://cdn.example.com/alice.png",
            receiverUserId = "receiver-1",
            receiverNickname = "Bob",
            receiverAvatar = null,          // ← 协议 receiver.avatar 可为 null
            giftId = "gift-1",
            giftCode = "castle_01",
            giftName = "قصر",
            giftIconUrl = "https://cdn.example.com/castle.png",
            giftAnimationUrl = "https://cdn.example.com/castle.svga",
            effectLevel = 4,
            count = 1,
            totalPrice = 520L,
        )
        assertEquals(
            "receiverAvatar 为 null 时应正确存储",
            null,
            eventWithNullAvatar.receiverAvatar,
        )

        // 构造时传入非 null 的 receiverAvatar
        val eventWithAvatar = GiftReceivedEvent(
            msgId = "msg-high1b",
            giftRecordId = "record-2",
            senderUserId = "sender-1",
            senderNickname = "Alice",
            senderAvatar = null,
            receiverUserId = "receiver-1",
            receiverNickname = "Bob",
            receiverAvatar = "https://cdn.example.com/bob.png",  // ← 非 null
            giftId = "gift-1",
            giftCode = "castle_01",
            giftName = "قصر",
            giftIconUrl = "https://cdn.example.com/castle.png",
            giftAnimationUrl = null,
            effectLevel = 4,
            count = 2,
            totalPrice = 1040L,
        )
        assertEquals(
            "receiverAvatar 非 null 时应正确存储",
            "https://cdn.example.com/bob.png",
            eventWithAvatar.receiverAvatar,
        )
    }

    // ─── MEDIUM-1：buildSendGiftJson 特殊字符 JSON 注入防护 ────────────────────

    /**
     * MEDIUM-1（Review R1）：buildSendGiftJson 使用字符串插值时，
     * giftId/recipientId 中含 `"` 或 `\` 会破坏 JSON 格式。
     *
     * RED 阶段：当前字符串插值实现遇到 `"` 字符时 JSON 解析将抛出异常。
     */
    @Test
    fun `MEDIUM-1 buildSendGiftJson with special characters in giftId produces valid parseable JSON`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // giftId 含双引号（最常见注入场景）
            val specialGiftId = "gift\"with\"quotes"
            val wsClient = FakeWebSocketClient()
            val gift = makeGift(id = specialGiftId, price = 10L)
            val repo = FakeGiftRepository(listGiftsResult = Result.success(listOf(gift)))
            val vm = GiftPanelViewModel(
                giftRepository = repo,
                wsClient = wsClient,
                roomId = "room-123",
            )
            advanceUntilIdle()

            vm.updateRecipients(listOf(makeMicUser("user-1")))
            vm.selectGift(specialGiftId)
            vm.selectCount(1)
            wsClient.simulateMessage(
                """{"type":"BalanceUpdated","msg_id":"b1","payload":{"diamond_balance":100,"delta":100,"reason":"recharge","ref_id":null},"timestamp":1720000000000}"""
            )
            advanceUntilIdle()
            wsClient.simulateConnect()

            assertTrue("含特殊字符的 giftId，canSend 仍应为 true", vm.uiState.value.canSend)

            vm.sendGift()
            runCurrent()

            assertTrue("应有 WS 消息发送", wsClient.sentMessages.isNotEmpty())
            val rawJson = wsClient.sentMessages.last()

            // 核心断言：JSON 必须可解析（字符串插值会在此处抛出异常）
            val parsed = try {
                JsonParser.parseString(rawJson).asJsonObject
            } catch (e: Exception) {
                throw AssertionError(
                    "含特殊字符的 giftId 导致 JSON 格式错误。\n原始 JSON: $rawJson",
                    e,
                )
            }

            val payload = parsed.getAsJsonObject("payload")
            assertNotNull("payload 不应为 null", payload)
            // gift_id 应被正确转义，原始值完整保留
            assertEquals(
                "gift_id 特殊字符应被正确转义并还原",
                specialGiftId,
                payload.get("gift_id").asString,
            )
        }
}
