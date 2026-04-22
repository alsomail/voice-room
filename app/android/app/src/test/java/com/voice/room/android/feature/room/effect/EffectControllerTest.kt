package com.voice.room.android.feature.room.effect

import com.voice.room.android.core.ws.event.GiftReceivedEvent
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
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
 * TDD 单元测试 — GiftEffectController (T-30031)
 *
 * 测试策略：
 * - 使用 backgroundScope 作为 controller scope（避免 UncompletedCoroutinesError）
 * - advanceUntilIdle() 在创建 controller 后调用，启动 l3Queue processor 协程
 * - L1/L2 在 onGiftReceived() 内同步更新，assert 无需 advance
 * - L3 由 l3Queue processor 协程设置，onGiftReceived() 后需 runCurrent() 才可见
 * - 时间推进用 advanceTimeBy() + runCurrent()
 *
 * E31-01: effect_level=1 → 仅 L1 弹幕，无 L2/L3
 * E31-02: effect_level=3 → L1 + L2 麦位光圈
 * E31-03: effect_level=5 → L1+L2+L3，L3 8s
 * E31-04: L3 播放期间再来一个 L3 → 进入队列，前一个结束后继续播放
 * E31-05: L3 点击跳过 → 立即结束（fullscreenEffect=null）
 * E31-06: 连击礼物 → L1 仅一条弹幕，count 累加
 * E31-07: isReplay=true → 仅 L1，无 L2/L3
 * E31-08: giftAnimationUrl=null → fallback 空字符串，不阻塞流程
 * E31-09: effect_level=4 → L3 duration=5000ms
 * E31-10: L2 光圈 2s 后自动清除
 * E31-11: 不同 sender+gift+receiver 组合 → 各自独立弹幕
 * E31-12: L3 队列最多 3 个，第 4 个丢弃
 * E31-13: isBold=true when effectLevel >= 3
 * E31-14: isBold=false when effectLevel < 3
 * E31-15: 跳过 L3 后排队事件继续播放
 */
@OptIn(ExperimentalCoroutinesApi::class)
class EffectControllerTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── 辅助函数 ─────────────────────────────────────────────────────────────

    private fun makeEvent(
        msgId: String = "msg-1",
        giftRecordId: String = "record-1",
        senderUserId: String = "sender-1",
        senderNickname: String = "Alice",
        senderAvatar: String? = null,
        receiverUserId: String = "receiver-1",
        receiverNickname: String = "Bob",
        receiverAvatar: String? = null,
        giftId: String = "gift-1",
        giftCode: String = "bouquet_01",
        giftName: String = "花束",
        giftIconUrl: String = "https://cdn.icon.png",
        giftAnimationUrl: String? = null,
        effectLevel: Int = 1,
        count: Int = 1,
        totalPrice: Long = 10L,
        isReplay: Boolean = false,
    ) = GiftReceivedEvent(
        msgId = msgId,
        giftRecordId = giftRecordId,
        senderUserId = senderUserId,
        senderNickname = senderNickname,
        senderAvatar = senderAvatar,
        receiverUserId = receiverUserId,
        receiverNickname = receiverNickname,
        receiverAvatar = receiverAvatar,
        giftId = giftId,
        giftCode = giftCode,
        giftName = giftName,
        giftIconUrl = giftIconUrl,
        giftAnimationUrl = giftAnimationUrl,
        effectLevel = effectLevel,
        count = count,
        totalPrice = totalPrice,
        isReplay = isReplay,
    )

    // ─── E31-01: effect_level=1 → 仅 L1 ─────────────────────────────────────

    @Test
    fun `E31-01 effect_level=1 only L1 barrage no L2 no L3`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle() // 启动 l3Queue processor 协程

            // onGiftReceived 是普通函数，同步执行 L1/L2 逻辑
            controller.onGiftReceived(makeEvent(effectLevel = 1))

            // L1: 弹幕新增一条（同步）
            assertEquals(1, controller.giftMessages.value.size)
            // L2: 无麦位光圈（effectLevel < 2）
            assertNull(controller.micGlowTargetUserId.value)
            // L3: 无全屏特效（effectLevel < 4）
            assertNull(controller.fullscreenEffect.value)
        }

    // ─── E31-02: effect_level=3 → L1 + L2 ───────────────────────────────────

    @Test
    fun `E31-02 effect_level=3 triggers L1 and L2 mic glow but no L3`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 3, receiverUserId = "user-bob"))
            // L1 和 L2 glow 同步设置，glow 定时清除 coroutine 已调度但未运行

            assertEquals(1, controller.giftMessages.value.size)
            assertEquals("user-bob", controller.micGlowTargetUserId.value)
            assertNull(controller.fullscreenEffect.value) // effectLevel=3 < 4，无 L3
        }

    // ─── E31-03: effect_level=5 → L1+L2+L3，8s ──────────────────────────────

    @Test
    fun `E31-03 effect_level=5 triggers all L1 L2 L3 with 8s duration`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(
                effectLevel = 5,
                receiverUserId = "user-bob",
                giftAnimationUrl = "https://cdn.anim.json",
            ))
            // L1/L2 已同步设置；l3Queue.trySend 触发了 l3Queue processor 被调度

            runCurrent() // 运行 l3Queue processor：playL3() → 设置 fullscreenEffect

            assertEquals(1, controller.giftMessages.value.size)
            assertEquals("user-bob", controller.micGlowTargetUserId.value)
            val fullscreen = controller.fullscreenEffect.value
            assertNotNull(fullscreen)
            assertEquals("https://cdn.anim.json", fullscreen!!.animationUrl)
            assertEquals(8_000L, fullscreen.durationMs)
        }

    // ─── E31-04: L3 播放中再来一个 → 排队 ─────────────────────────────────────

    @Test
    fun `E31-04 second L3 event queued and plays after first ends`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(
                msgId = "msg-first",
                effectLevel = 4,
                giftAnimationUrl = "https://anim1.json",
            ))
            runCurrent() // l3Queue processor 开始播放第一个

            assertNotNull(controller.fullscreenEffect.value)
            assertEquals("https://anim1.json", controller.fullscreenEffect.value!!.animationUrl)

            // 发送第二个 L3 → 进入 l3Queue buffer（processor 仍在等 join）
            controller.onGiftReceived(makeEvent(
                msgId = "msg-second",
                effectLevel = 4,
                giftAnimationUrl = "https://anim2.json",
            ))

            // 第一个仍在播放（时间未推进）
            assertEquals("https://anim1.json", controller.fullscreenEffect.value!!.animationUrl)

            // 推进 5001ms → 第一个 delay 完成
            advanceTimeBy(5_001L)
            runCurrent() // processor 恢复：清 null，拿第二个，开始播放

            assertNotNull(controller.fullscreenEffect.value)
            assertEquals("https://anim2.json", controller.fullscreenEffect.value!!.animationUrl)
        }

    // ─── E31-05: 点击跳过立即结束 ────────────────────────────────────────────

    @Test
    fun `E31-05 skip fullscreen immediately clears fullscreenEffect`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 4, giftAnimationUrl = "https://anim.json"))
            runCurrent()

            assertNotNull(controller.fullscreenEffect.value)

            controller.skipFullscreen() // 同步：cancel delay + set null

            assertNull(controller.fullscreenEffect.value)
        }

    // ─── E31-06: 连击礼物 → count 累加 ───────────────────────────────────────

    @Test
    fun `E31-06 combo gifts accumulate count in single barrage entry`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(msgId = "msg-1", senderUserId = "alice", giftId = "castle", receiverUserId = "bob", count = 1))
            controller.onGiftReceived(makeEvent(msgId = "msg-2", senderUserId = "alice", giftId = "castle", receiverUserId = "bob", count = 1))

            assertEquals(1, controller.giftMessages.value.size)
            assertEquals(2, controller.giftMessages.value[0].count)
        }

    // ─── E31-07: isReplay=true → 仅 L1 ──────────────────────────────────────

    @Test
    fun `E31-07 replay event triggers only L1 no L2 no L3`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(
                effectLevel = 5,
                isReplay = true,
                receiverUserId = "user-bob",
                giftAnimationUrl = "https://cdn.anim.json",
            ))
            runCurrent() // 确认 l3Queue processor 没收到任何内容

            assertEquals(1, controller.giftMessages.value.size)
            assertNull(controller.micGlowTargetUserId.value)
            assertNull(controller.fullscreenEffect.value)
        }

    // ─── E31-08: animation_url=null → fallback 空字符串 ──────────────────────

    @Test
    fun `E31-08 null animation_url uses empty string fallback without blocking`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 4, giftAnimationUrl = null))
            runCurrent()

            val fullscreen = controller.fullscreenEffect.value
            assertNotNull(fullscreen)
            assertEquals("", fullscreen!!.animationUrl)
        }

    // ─── E31-09: effect_level=4 → 5s duration ───────────────────────────────

    @Test
    fun `E31-09 effect_level=4 L3 duration is 5000ms`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 4, giftAnimationUrl = "https://anim.json"))
            runCurrent()

            assertEquals(5_000L, controller.fullscreenEffect.value!!.durationMs)
        }

    // ─── E31-10: L2 光圈 2s 后自动清除 ──────────────────────────────────────

    @Test
    fun `E31-10 L2 mic glow auto-clears after 2000ms`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 2, receiverUserId = "user-alice"))
            // glow 同步设置

            assertEquals("user-alice", controller.micGlowTargetUserId.value)

            advanceTimeBy(2_001L)
            runCurrent() // glow cleaner 运行

            assertNull(controller.micGlowTargetUserId.value)
        }

    // ─── E31-11: 不同组合 → 各自独立弹幕 ────────────────────────────────────

    @Test
    fun `E31-11 different gift combos produce separate barrage entries`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(msgId = "msg-1", senderUserId = "alice", giftId = "castle", receiverUserId = "bob"))
            controller.onGiftReceived(makeEvent(msgId = "msg-2", senderUserId = "alice", giftId = "rose", receiverUserId = "bob"))
            controller.onGiftReceived(makeEvent(msgId = "msg-3", senderUserId = "charlie", giftId = "castle", receiverUserId = "bob"))

            assertEquals(3, controller.giftMessages.value.size)
        }

    // ─── E31-12: L3 队列最多 3 个，第 4 个丢弃 ──────────────────────────────

    @Test
    fun `E31-12 L3 queue capacity is 3 excess events are dropped`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            // 第 1 个：l3Queue processor 拿走并开始播放
            controller.onGiftReceived(makeEvent(msgId = "msg-1", effectLevel = 4, giftAnimationUrl = "https://anim-1.json"))
            runCurrent() // processor 开始播放第 1 个

            // 第 2、3、4 个：进入 l3Queue buffer（容量=3）
            controller.onGiftReceived(makeEvent(msgId = "msg-2", effectLevel = 4, giftAnimationUrl = "https://anim-2.json"))
            controller.onGiftReceived(makeEvent(msgId = "msg-3", effectLevel = 4, giftAnimationUrl = "https://anim-3.json"))
            controller.onGiftReceived(makeEvent(msgId = "msg-4", effectLevel = 4, giftAnimationUrl = "https://anim-4.json"))
            // buffer 已满 [2, 3, 4]

            // 第 5 个：trySend 失败，静默丢弃
            controller.onGiftReceived(makeEvent(msgId = "msg-5", effectLevel = 4, giftAnimationUrl = "https://anim-5.json"))

            // 播完第 1 个
            advanceTimeBy(5_001L); runCurrent()
            // 播完第 2 个
            advanceTimeBy(5_001L); runCurrent()
            // 播完第 3 个
            advanceTimeBy(5_001L); runCurrent()
            // 播完第 4 个
            advanceTimeBy(5_001L); runCurrent()

            // 第 5 个已丢弃，队列空，特效结束
            advanceTimeBy(1_000L); runCurrent()
            assertNull(controller.fullscreenEffect.value)
        }

    // ─── E31-13: isBold=true when effectLevel >= 3 ───────────────────────────

    @Test
    fun `E31-13 gift message isBold=true when effectLevel=3`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 3))

            assertTrue(controller.giftMessages.value[0].isBold)
        }

    // ─── E31-14: isBold=false when effectLevel < 3 ───────────────────────────

    @Test
    fun `E31-14 gift message isBold=false when effectLevel=1`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(effectLevel = 1))

            assertFalse(controller.giftMessages.value[0].isBold)
        }

    // ─── E31-15: 跳过后排队事件继续播放 ─────────────────────────────────────

    @Test
    fun `E31-15 after skip queued L3 event plays next`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val controller = GiftEffectController(backgroundScope)
            advanceUntilIdle()

            controller.onGiftReceived(makeEvent(msgId = "first", effectLevel = 4, giftAnimationUrl = "https://first.json"))
            runCurrent() // 第一个开始播放

            controller.onGiftReceived(makeEvent(msgId = "second", effectLevel = 4, giftAnimationUrl = "https://second.json"))

            assertEquals("https://first.json", controller.fullscreenEffect.value!!.animationUrl)

            controller.skipFullscreen() // 同步：cancel delay + set null
            assertNull(controller.fullscreenEffect.value)

            runCurrent() // l3Queue processor 恢复并开始第二个

            assertNotNull(controller.fullscreenEffect.value)
            assertEquals("https://second.json", controller.fullscreenEffect.value!!.animationUrl)
        }
}
