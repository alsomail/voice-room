package com.voice.room.android.core.ws

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — FakeWebSocketClient
 *
 * 验证 Fake 实现的状态机行为，供 ViewModel / 上层逻辑测试使用。
 *   F-01: 初始状态为 Disconnected
 *   F-02: connect() 经过 Connecting → Connected
 *   F-03: simulateMessage() 发射 Message(text)
 *   F-04: simulateDisconnect() 发射 Disconnected
 *   F-05: send() 在 Connected 状态追加到 sentMessages
 *   F-06: send() 在 Disconnected 状态不追加，不抛异常
 *   F-07: disconnect() 设置 Disconnected("manual")，不抛异常
 *   F-08: sentMessages 记录顺序与调用顺序一致
 *   F-09: 多次 connect() 每次都经过 Connecting → Connected
 */
@OptIn(ExperimentalCoroutinesApi::class)
class FakeWebSocketClientTest {

    // ─── F-01: 初始状态为 Disconnected ────────────────────────────────────────

    @Test
    fun `F-01 initial state is Disconnected`() {
        val fake = FakeWebSocketClient()
        assertEquals(WebSocketState.Disconnected(), fake.state.value)
    }

    // ─── F-02: connect() 经历 Connecting → Connected ─────────────────────────

    @Test
    fun `F-02 connect emits Connecting then Connected`() =
        // UnconfinedTestDispatcher runs coroutines eagerly so the collector
        // sees every intermediate StateFlow emission (Connecting AND Connected)
        runTest(UnconfinedTestDispatcher()) {
            val fake = FakeWebSocketClient()
            val states = mutableListOf<WebSocketState>()

            val job = launch { fake.state.collect { states.add(it) } }

            fake.connect(
                RoomSocketRequestSpec(
                    url = "ws://fake-host/ws",
                    headers = mapOf("Authorization" to "Bearer fake-token")
                )
            )
            advanceUntilIdle()
            job.cancel()

            assertTrue("Should contain Connecting", states.contains(WebSocketState.Connecting))
            assertTrue("Should contain Connected", states.any { it is WebSocketState.Connected })

            val connectingIdx = states.indexOf(WebSocketState.Connecting)
            val connectedIdx = states.indexOfFirst { it is WebSocketState.Connected }
            assertTrue("Connecting must come before Connected", connectingIdx < connectedIdx)
        }

    // ─── F-03: simulateMessage() 发射 Message(text) ──────────────────────────

    @Test
    fun `F-03 simulateMessage emits Message state`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        fake.simulateMessage("""{"type":"user_joined"}""")

        val state = fake.state.value
        assertTrue("State must be Message", state is WebSocketState.Message)
        assertEquals("""{"type":"user_joined"}""", (state as WebSocketState.Message).text)
    }

    // ─── F-04: simulateDisconnect() 发射 Disconnected ────────────────────────

    @Test
    fun `F-04 simulateDisconnect emits Disconnected with reason`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        fake.simulateDisconnect("server restarted")

        val state = fake.state.value
        assertTrue("State must be Disconnected", state is WebSocketState.Disconnected)
        assertEquals("server restarted", (state as WebSocketState.Disconnected).reason)
    }

    // ─── F-05: send() 在 Connected 追加到 sentMessages ───────────────────────

    @Test
    fun `F-05 send when Connected appends to sentMessages`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        fake.send("""{"type":"join"}""")
        fake.send("""{"type":"mic_on"}""")

        assertEquals(2, fake.sentMessages.size)
        assertEquals("""{"type":"join"}""", fake.sentMessages[0])
        assertEquals("""{"type":"mic_on"}""", fake.sentMessages[1])
    }

    // ─── F-06: send() 在 Disconnected 不追加、不抛异常 ───────────────────────

    @Test
    fun `F-06 send when Disconnected silently drops message without throwing`() {
        val fake = FakeWebSocketClient()
        // Initial state is Disconnected

        try {
            fake.send("""{"type":"join"}""")
        } catch (e: Exception) {
            throw AssertionError("send() must not throw in Disconnected state: $e")
        }

        assertTrue("sentMessages must be empty in Disconnected state", fake.sentMessages.isEmpty())
    }

    // ─── F-07: disconnect() 设置 Disconnected("manual") ─────────────────────

    @Test
    fun `F-07 disconnect sets Disconnected manual reason without throwing`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        try {
            fake.disconnect()
        } catch (e: Exception) {
            throw AssertionError("disconnect() must not throw: $e")
        }

        val state = fake.state.value
        assertTrue("State must be Disconnected", state is WebSocketState.Disconnected)
        assertEquals("manual", (state as WebSocketState.Disconnected).reason)
    }

    // ─── F-08: sentMessages 记录顺序与调用顺序一致 ────────────────────────────

    @Test
    fun `F-08 sentMessages preserves insertion order`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        val messages = listOf("A", "B", "C", "D", "E")
        messages.forEach { fake.send(it) }

        assertEquals(messages, fake.sentMessages.toList())
    }

    // ─── F-09: 多次 connect() 每次都经历 Connecting → Connected ──────────────

    @Test
    fun `F-09 repeated connect calls always reach Connected`() = runTest {
        val fake = FakeWebSocketClient()
        val spec = RoomSocketRequestSpec("ws://fake/ws", emptyMap())

        repeat(3) {
            fake.connect(spec)
            assertTrue(
                "After connect #$it state must be Connected",
                fake.state.value is WebSocketState.Connected
            )
        }
    }

    // ─── F-10: simulateError() 发射 Error 状态 ───────────────────────────────

    @Test
    fun `F-10 simulateError emits Error state`() {
        val fake = FakeWebSocketClient()
        val cause = RuntimeException("network failure")

        fake.simulateError(cause)

        val state = fake.state.value
        assertTrue("State must be Error", state is WebSocketState.Error)
        assertEquals(cause, (state as WebSocketState.Error).throwable)
    }

    // ─── F-11: connect() 后 sentMessages 不因 disconnect 清除 ────────────────

    @Test
    fun `F-11 sentMessages persists after disconnect`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        fake.send("msg-1")
        fake.disconnect()

        assertEquals(1, fake.sentMessages.size)
        assertEquals("msg-1", fake.sentMessages[0])
    }

    // ─── F-12: 初始 send() 返回 false (Disconnected) ─────────────────────────

    @Test
    fun `F-12 send returns false when not Connected`() {
        val fake = FakeWebSocketClient()
        assertFalse("send should return false when Disconnected", fake.send("hello"))
    }

    // ─── F-13: send() 在 Connected 返回 true ─────────────────────────────────

    @Test
    fun `F-13 send returns true when Connected`() = runTest {
        val fake = FakeWebSocketClient()
        fake.connect(RoomSocketRequestSpec("ws://fake/ws", emptyMap()))

        assertTrue("send should return true when Connected", fake.send("hello"))
    }
}
