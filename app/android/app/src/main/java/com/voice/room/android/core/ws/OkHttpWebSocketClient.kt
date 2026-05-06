package com.voice.room.android.core.ws

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import okhttp3.OkHttpClient
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

// ─── 异常 ──────────────────────────────────────────────────────────────────────

/**
 * 当 WebSocket 重连次数超过 [OkHttpWebSocketClient.maxRetries] 时抛出。
 */
class MaxRetryExceededException(retries: Int) :
    Exception("WebSocket max retry exceeded after $retries attempts")

// ─── 实现 ──────────────────────────────────────────────────────────────────────

/**
 * 基于 OkHttp 4.x 的 WebSocket 客户端实现。
 *
 * 功能特性：
 * - **状态流广播**：所有连接状态通过 [state] StateFlow 实时推送
 * - **心跳保活**：连接成功后每 [pingIntervalMs] 毫秒发送 `{"type":"Ping"}`；
 *   收到 `{"type":"Pong"}` 后重置计时器
 * - **自动重连**：非主动断开时以指数退避（1s→2s→4s→8s→16s→32s，上限 32s）
 *   最多重试 [maxRetries] 次；超限后发出 [WebSocketState.Error]
 * - **防并发重入**：使用 [AtomicBoolean] 保证同时只有一个重连任务在进行
 *
 * @param okHttpClient  预配置好的 OkHttpClient（无需额外设置 pingInterval，由本类管理）
 * @param scope         协程作用域（传入 TestScope 可对 delay 使用虚拟时间）
 * @param pingIntervalMs 心跳间隔毫秒数，默认 15 000（15s）
 * @param maxRetries    最大重试次数，默认 8
 */
class OkHttpWebSocketClient(
    private val okHttpClient: OkHttpClient,
    private val scope: CoroutineScope,
    private val pingIntervalMs: Long = 15_000L,
    private val maxRetries: Int = 8,
) : IWebSocketClient {

    companion object {
        private const val TAG = "OkHttpWebSocketClient"
    }

    private val _state = MutableStateFlow<WebSocketState>(WebSocketState.Disconnected())
    override val state: StateFlow<WebSocketState> = _state.asStateFlow()

    @Volatile private var webSocket: WebSocket? = null
    @Volatile private var isManualDisconnect = false
    @Volatile private var currentSpec: RoomSocketRequestSpec? = null
    /** Separate flag so heartbeat doesn't break when state transitions to Message */
    @Volatile private var isConnectionActive = false

    private val retryCount = AtomicInteger(0)
    private val isReconnecting = AtomicBoolean(false)

    // HIGH-1 FIX: @Volatile 保证跨线程（OkHttp 线程 ↔ 调用者线程）可见性
    // onOpen/onMessage 在 OkHttp 内部线程调用 startHeartbeat() 写入 pingJob，
    // disconnect() 可能在任意线程调用 stopHeartbeat() 读取 pingJob。
    @Volatile private var pingJob: Job? = null

    // ─── 公开 API ──────────────────────────────────────────────────────────────

    override suspend fun connect(spec: RoomSocketRequestSpec) {
        isManualDisconnect = false
        isConnectionActive = false
        retryCount.set(0)
        isReconnecting.set(false)
        currentSpec = spec
        doConnect(spec)
    }

    override fun send(message: String): Boolean {
        if (!isConnectionActive) return false
        return webSocket?.send(message) ?: false
    }

    override fun disconnect() {
        isManualDisconnect = true
        isConnectionActive = false
        stopHeartbeat()
        webSocket?.close(1000, "manual")
        // MEDIUM-1 FIX: cancel() 立即强制释放底层 Socket 资源；
        // close() 仅发关闭握手并等待服务端回应（最长 60s），若服务端无响应会占用资源。
        webSocket?.cancel()
        webSocket = null
        _state.value = WebSocketState.Disconnected("manual")
    }

    // ─── 内部逻辑 ──────────────────────────────────────────────────────────────

    private fun doConnect(spec: RoomSocketRequestSpec) {
        _state.value = WebSocketState.Connecting
        webSocket = okHttpClient.newWebSocket(spec.toOkHttpRequest(), createListener())
    }

    private fun createListener() = object : WebSocketListener() {

        override fun onOpen(webSocket: WebSocket, response: Response) {
            retryCount.set(0)
            isReconnecting.set(false)
            isConnectionActive = true
            _state.value = WebSocketState.Connected
            startHeartbeat()
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            // T-30051: WS 接收链路可观测性 — 节点 1（onMessage 入口）。
            // 仅打印 head 80 字符（PII 保护）。
            Log.i(TAG, "ws: received text len=${text.length}, head=${text.take(80)}")
            if (text.contains("\"type\":\"Pong\"")) {
                // Pong 重置心跳计时：取消当前 pingJob，重新开始倒计时
                startHeartbeat()
            }
            _state.value = WebSocketState.Message(text)
        }

        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
            webSocket.close(1000, null)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            isConnectionActive = false
            stopHeartbeat()
            if (isManualDisconnect) {
                _state.value = WebSocketState.Disconnected("manual")
            } else {
                _state.value = WebSocketState.Disconnected(reason)
                scheduleReconnect()
            }
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            // MEDIUM-2 FIX: 记录连接失败日志，生产环境定位问题时可追溯
            Log.w(TAG, "WebSocket onFailure (manual=$isManualDisconnect): ${t.message}", t)
            isConnectionActive = false
            stopHeartbeat()
            if (!isManualDisconnect) {
                scheduleReconnect()
            }
            // isManualDisconnect=true 时：onFailure 可能由 disconnect() 中的 cancel() 触发，
            // 状态已由 disconnect() 设置为 Disconnected("manual")，此处不再覆盖为 Error。
        }
    }

    /**
     * 启动（或重启）心跳协程。
     * 每 [pingIntervalMs] 后发送一次 ping；收到 pong 时重新调用本函数以重置计时。
     * 使用 [isConnectionActive] 而非 state 判断，避免 Message 状态干扰。
     */
    private fun startHeartbeat() {
        pingJob?.cancel()
        pingJob = scope.launch {
            while (true) {
                delay(pingIntervalMs)
                if (isConnectionActive) {
                    webSocket?.send(WsEnvelope.build("Ping"))
                } else {
                    break
                }
            }
        }
    }

    private fun stopHeartbeat() {
        pingJob?.cancel()
        pingJob = null
    }

    /**
     * 以指数退避调度重连。
     * 使用 [AtomicBoolean] 防止并发 onFailure / onClosed 同时触发多条重连。
     *
     * 退避表：retry 1→1s, 2→2s, 3→4s, 4→8s, 5→16s, 6+→32s
     */
    private fun scheduleReconnect() {
        // CAS 防重入：只有首个调用者进入，其余直接返回
        if (!isReconnecting.compareAndSet(false, true)) return

        scope.launch {
            try {
                if (isManualDisconnect) return@launch

                val currentRetry = retryCount.get()
                if (currentRetry >= maxRetries) {
                    // MEDIUM-2 FIX: 超出最大重试次数时记录错误日志
                    Log.e(TAG, "WebSocket max retry exceeded ($currentRetry attempts)")
                    _state.value = WebSocketState.Error(MaxRetryExceededException(currentRetry))
                    return@launch
                }

                // 指数退避：1s, 2s, 4s, 8s, 16s, 32s（超过 5 次后固定 32s）
                val backoffSec = minOf(1L shl currentRetry, 32L)
                retryCount.incrementAndGet()
                // MEDIUM-2 FIX: 记录重连调度日志
                Log.w(TAG, "Scheduling reconnect #${retryCount.get()}/$maxRetries, backoff=${backoffSec}s")
                delay(backoffSec * 1_000L)

                if (!isManualDisconnect) {
                    isReconnecting.set(false)  // 允许下一次重连触发（若本次也失败）
                    currentSpec?.let { doConnect(it) }
                }
            } catch (e: CancellationException) {
                // HIGH-2 FIX: 必须重新抛出 CancellationException，遵守 Kotlin 结构化并发协议。
                // 吞噬 CancellationException 会阻止父 scope 正确取消所有子协程，
                // 导致 App 关闭 / 生命周期切换时资源清理被延迟。
                isReconnecting.set(false)
                throw e
            } catch (_: Exception) {
                isReconnecting.set(false)
            }
        }
    }
}
