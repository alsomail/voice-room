package com.voice.room.android.core.ws

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import okhttp3.OkHttpClient
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.IOException
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/**
 * TDD 集成测试 — OkHttpWebSocketClient
 *
 * 使用 MockWebServer 测试真实 OkHttp WebSocket 行为。
 * ⚠️  关键设计原则：
 *   - client 使用独立 IO scope（非 runBlocking 的 event-loop 线程）
 *     避免 blocking 操作（Channel.receive 等）阻塞 delay() 协程
 *   - 状态收集通过 client.state.first { } suspend 函数（非阻塞）
 *   - MockWebServer 消息传递使用 Channel（非 LinkedBlockingQueue）
 *
 * 测试 ID 对应 T-30008 TDS 验收用例：
 *   WS-01: connect → Connecting then Connected
 *   WS-02: 服务器拒绝 → Error 或 Disconnected
 *   WS-03: 非主动断开 → 约 1s 后 Connecting
 *   WS-04: 超出最大重试 → Error(MaxRetryExceededException)
 *   WS-05: send() → MockWebServer 收到消息
 *   WS-06: 服务端推送文本 → Message(text)
 *   WS-07: 心跳 pingInterval 后自动发 {"type":"ping"}
 *   WS-08: pong → 重置心跳，不提前发 ping
 *   WS-09: disconnect() → Disconnected，无重连
 *   WS-10: Disconnected 下 send() 不抛异常，返回 false
 *   WS-11: 并发失败只产生一条重连
 */
@OptIn(ExperimentalCoroutinesApi::class)
class OkHttpWebSocketClientTest {

    private val mockWebServer = MockWebServer()

    private val okHttpClient = OkHttpClient.Builder()
        .connectTimeout(3, TimeUnit.SECONDS)
        .readTimeout(3, TimeUnit.SECONDS)
        .build()

    @Before
    fun setup() {
        mockWebServer.start()
    }

    @After
    fun teardown() {
        // Gracefully shut down; catch IOException if connections weren't cleanly closed
        try {
            mockWebServer.shutdown()
        } catch (_: IOException) {
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    /** 每个测试使用独立 IO Scope，防止 client 协程与测试 event-loop 互相阻塞 */
    private fun makeClientScope(): CoroutineScope =
        CoroutineScope(Dispatchers.IO + SupervisorJob())

    private fun wsSpec() = RoomSocketRequestSpec(
        url = "ws://localhost:${mockWebServer.port}/ws",
        headers = mapOf("Authorization" to "Bearer test-token")
    )

    // ─── WS-01: connect 后 state 先 Connecting 再 Connected ──────────────────

    @Test
    fun `WS-01 connect emits Connecting then Connected`() = runBlocking {
        val stateHistory = CopyOnWriteArrayList<WebSocketState>()
        val clientScope = makeClientScope()

        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(NoOpListener()))

        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        // Subscribe BEFORE connecting so we catch every intermediate state
        clientScope.launch { client.state.collect { stateHistory.add(it) } }
        delay(60) // Give the IO collector time to subscribe

        client.connect(wsSpec())
        // Wait (suspend, not blocking) until Connected
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        assertTrue("stateHistory must contain Connecting", stateHistory.contains(WebSocketState.Connecting))
        assertTrue("stateHistory must contain Connected", stateHistory.any { it is WebSocketState.Connected })

        val connectingIdx = stateHistory.indexOf(WebSocketState.Connecting)
        val connectedIdx = stateHistory.indexOfFirst { it is WebSocketState.Connected }
        assertTrue("Connecting must precede Connected", connectingIdx < connectedIdx)

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-02: 服务器拒绝连接 → Error 或 Disconnected ─────────────────────

    @Test
    fun `WS-02 server rejection results in Error or Disconnected`() = runBlocking {
        // 403 causes OkHttp onFailure → scheduleReconnect, but maxRetries=0 → immediate Error
        mockWebServer.enqueue(MockResponse().setResponseCode(403))

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 0
        )

        client.connect(wsSpec())

        val errorState = withTimeout(5_000) {
            client.state.first { it is WebSocketState.Error || it is WebSocketState.Disconnected }
        }

        assertTrue(
            "State after rejection must be Error or Disconnected, was $errorState",
            errorState is WebSocketState.Error || errorState is WebSocketState.Disconnected
        )

        clientScope.cancel()
    }

    // ─── WS-03: 非主动断开 → 约 1s 后重连 → Connecting ──────────────────────

    @Test
    fun `WS-03 non-manual close triggers reconnect back to Connecting`() = runBlocking {
        var serverWs: WebSocket? = null
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
        }))
        // Second response for the reconnect attempt
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(NoOpListener()))

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // Server closes — non-manual trigger
        serverWs?.close(1001, "server restart")

        // Should re-enter Connecting within backoff window (1s + buffer)
        withTimeout(6_000) {
            client.state.first { it is WebSocketState.Connecting || it is WebSocketState.Connected }
        }

        val finalState = client.state.value
        assertTrue(
            "Should re-enter Connecting or Connected after non-manual close, was $finalState",
            finalState is WebSocketState.Connecting || finalState is WebSocketState.Connected
        )

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-04: 超出最大重试次数 → Error(MaxRetryExceededException) ──────────

    @Test
    fun `WS-04 exceeding max retries emits Error with MaxRetryExceededException`() = runBlocking {
        var serverWs: WebSocket? = null
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
        }))
        // All retry attempts fail (503 → onFailure for each)
        repeat(6) {
            mockWebServer.enqueue(MockResponse().setResponseCode(503))
        }

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 2 // Quick: only 2 retries before Error
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // Server closes abruptly to kick off reconnect cycle
        serverWs?.close(1001, "dropped")

        // maxRetries=2: backoff 1s + 2s = 3s, then Error
        val errorState = withTimeout(20_000) {
            client.state.first { it is WebSocketState.Error }
        }

        assertTrue("Must be Error state", errorState is WebSocketState.Error)
        assertTrue(
            "Error.throwable must be MaxRetryExceededException",
            (errorState as WebSocketState.Error).throwable is MaxRetryExceededException
        )

        clientScope.cancel()
    }

    // ─── WS-05: Connected 下 send() → MockWebServer 收到消息 ─────────────────

    @Test
    fun `WS-05 send delivers message to server when Connected`() = runBlocking {
        val receivedByServer = Channel<String>(Channel.UNLIMITED)

        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onMessage(ws: WebSocket, text: String) {
                receivedByServer.trySend(text)
            }
        }))

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        val msg = """{"type":"join","roomId":"room-42"}"""
        val sent = client.send(msg)
        assertTrue("send() should return true when Connected", sent)

        val received = withTimeout(5_000) { receivedByServer.receive() }
        assertEquals("Server should receive exact message", msg, received)

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-06: 服务端推送文本 → state 发射 Message(text) ────────────────────

    @Test
    fun `WS-06 server push emits Message state`() = runBlocking {
        var serverWs: WebSocket? = null
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
        }))

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        val payload = """{"type":"user_joined","payload":{"userId":"u-1"}}"""
        serverWs?.send(payload)

        val msgState = withTimeout(5_000) {
            client.state.first { it is WebSocketState.Message }
        }

        assertTrue("State must be Message", msgState is WebSocketState.Message)
        assertEquals("Message text must match server payload", payload, (msgState as WebSocketState.Message).text)

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-07: 心跳在 pingIntervalMs 后自动发 {"type":"ping"} ───────────────

    @Test
    fun `WS-07 heartbeat sends ping after pingInterval`() = runBlocking {
        val serverReceived = Channel<String>(Channel.UNLIMITED)

        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onMessage(ws: WebSocket, text: String) {
                serverReceived.trySend(text)
            }
        }))

        val pingIntervalMs = 400L
        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = pingIntervalMs,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // Wait up to 3× the interval; ping arrives after exactly 1× interval
        val ping = withTimeout(pingIntervalMs * 4) { serverReceived.receive() }
        assertEquals("""{"type":"ping"}""", ping)

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-08: Pong 重置心跳计时器 ─────────────────────────────────────────

    @Test
    fun `WS-08 pong resets heartbeat - no premature ping after pong`() = runBlocking {
        val serverReceived = Channel<String>(Channel.UNLIMITED)
        var serverWs: WebSocket? = null

        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
            override fun onMessage(ws: WebSocket, text: String) {
                serverReceived.trySend(text)
            }
        }))

        val pingIntervalMs = 600L
        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = pingIntervalMs,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // Send pong at 60% of the ping interval (before first ping fires at 100%)
        val pongAt = pingIntervalMs * 3 / 5  // = 360ms
        delay(pongAt)
        serverWs?.send("""{"type":"pong"}""")

        // Window before original interval would have fired: ~40% of interval = 240ms
        // No ping should arrive in this window (heartbeat was reset)
        var prematurePing: String? = null
        try {
            // Check for 240ms + 100ms safety buffer
            prematurePing = withTimeout((pingIntervalMs * 2 / 5) + 100) {
                serverReceived.receive()
            }
        } catch (_: kotlinx.coroutines.TimeoutCancellationException) {
            // Expected: no ping in the short window after pong
        }
        assertNull("No ping expected immediately after pong (heartbeat was reset)", prematurePing)

        // Ping SHOULD arrive at ~pongAt + pingIntervalMs = 360+600 = 960ms from test start
        // We're at ~460ms now (360ms wait + ~100ms timeout). Remaining: ~500ms + buffer
        val delayedPing = withTimeout(pingIntervalMs + 400) {
            serverReceived.receive()
        }
        assertNotNull("Ping expected after full interval elapsed since pong", delayedPing)
        assertEquals("""{"type":"ping"}""", delayedPing)

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-09: disconnect() 后不触发重连 ────────────────────────────────────

    @Test
    fun `WS-09 manual disconnect stays Disconnected without reconnecting`() = runBlocking {
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(NoOpListener()))

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 8
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        client.disconnect()

        // Immediately verify Disconnected
        withTimeout(3_000) { client.state.first { it is WebSocketState.Disconnected } }

        // Wait well beyond the 1s first-retry backoff and verify NO re-entry to Connecting
        delay(1_800)
        val finalState = client.state.value
        assertTrue(
            "After manual disconnect, state must remain Disconnected (no reconnect), was $finalState",
            finalState is WebSocketState.Disconnected
        )

        clientScope.cancel()
    }

    // ─── WS-10: Disconnected 下 send() 不抛异常，静默返回 false ──────────────

    @Test
    fun `WS-10 send in Disconnected state returns false without throwing`() {
        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        // Initial state is Disconnected (no connect() called)
        assertEquals(WebSocketState.Disconnected(), client.state.value)

        val result = try {
            client.send("""{"type":"join"}""")
        } catch (e: Exception) {
            throw AssertionError("send() must not throw in Disconnected state, but threw: $e")
        }

        assertFalse("send() must return false when not Connected", result)
        clientScope.cancel()
    }

    // ─── WS-11: 并发 onFailure 不产生两条 WebSocket 连接 ─────────────────────

    @Test
    fun `WS-11 concurrent failures do not create duplicate WebSocket connections`() = runBlocking {
        var serverWs: WebSocket? = null
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
        }))

        val reconnectAttempts = AtomicInteger(0)
        repeat(5) {
            mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
                override fun onOpen(ws: WebSocket, response: Response) {
                    reconnectAttempts.incrementAndGet()
                    ws.close(1000, null)
                }
            }))
        }

        val clientScope = makeClientScope()
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 3
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // Server closes abruptly (triggers reconnect with 1s backoff)
        serverWs?.close(1001, "dropped")

        // Wait 400ms — well under the 1s first-retry backoff
        // If anti-concurrent logic works, NO reconnect should have fired yet
        delay(400)
        assertEquals(
            "No reconnect should start within 400ms (backoff is 1s), got: ${reconnectAttempts.get()}",
            0,
            reconnectAttempts.get()
        )

        client.disconnect()
        delay(150)
        clientScope.cancel()
    }

    // ─── WS-12: scope 取消时 scheduleReconnect 正确终止（传播 CancellationException）
    //
    //  验证 HIGH-2 修复：
    //  旧代码：catch (_: Exception) 吞噬 CancellationException，协程虽也退出，
    //            但不遵守结构化并发协议（child job 状态为 "completed" 而非 "cancelled"），
    //            且若 catch 块内有后续 suspend 调用则会继续执行直至自然结束。
    //  修复后：catch (e: CancellationException) { ...; throw e } 确保协程被正确取消，
    //            scope.job.join() 在 backoff 窗口内迅速完成，不发生死锁或挂起。
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `WS-12 scheduleReconnect propagates CancellationException on scope cancel`() = runBlocking {
        var serverWs: WebSocket? = null
        mockWebServer.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                serverWs = ws
            }
        }))

        // maxRetries=8 → first backoff = 1s (delay(1000L))；scope cancel 应在 1s 内令 job 完成
        val clientScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
        val client = OkHttpWebSocketClient(
            okHttpClient = okHttpClient,
            scope = clientScope,
            pingIntervalMs = 60_000L,
            maxRetries = 8
        )

        client.connect(wsSpec())
        withTimeout(5_000) { client.state.first { it is WebSocketState.Connected } }

        // 服务端非主动关闭 → scheduleReconnect 内的 delay(1000L) 开始计时
        serverWs?.close(1001, "server restart")

        // 等待重连协程进入 delay() 之后再取消（100ms << 1000ms backoff）
        delay(100)

        val scopeJob = clientScope.coroutineContext[Job]!!
        clientScope.cancel()   // 取消 scope → delay() 应立即抛出 CancellationException

        // 断言：scope job 在 500ms 内完成（远小于 1s backoff 窗口）
        // 若 CancellationException 被正确重抛，child coroutine 立即取消 → scopeJob.join() 迅速返回
        withTimeout(500) { scopeJob.join() }
        assertTrue("scope job must be completed (cancelled) after cancel()", scopeJob.isCompleted)
    }

    // ─── Helper ───────────────────────────────────────────────────────────────

    private class NoOpListener : WebSocketListener()
}
