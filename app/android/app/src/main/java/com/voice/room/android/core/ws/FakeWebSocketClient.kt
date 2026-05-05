package com.voice.room.android.core.ws

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 测试用 [IWebSocketClient] Fake 实现。
 *
 * 提供完整的状态机行为模拟，供 ViewModel 单元测试和集成测试使用，
 * 无需真实网络连接。
 *
 * 用法示例：
 * ```kotlin
 * val fake = FakeWebSocketClient()
 * fake.connect(spec)                      // Connecting → Connected
 * fake.simulateMessage("""{"type":"x"}""") // 发射 Message
 * fake.simulateDisconnect("reason")        // 发射 Disconnected
 * fake.simulateError(IOException())        // 发射 Error
 * println(fake.sentMessages)               // 检查发送的消息
 * ```
 */
class FakeWebSocketClient : IWebSocketClient {

    private val _state = MutableStateFlow<WebSocketState>(WebSocketState.Disconnected())
    override val state: StateFlow<WebSocketState> = _state.asStateFlow()

    /** 记录所有通过 [send] 发出的消息，按调用顺序排列。 */
    val sentMessages = mutableListOf<String>()

    /**
     * [connect] 被调用的次数（TC-WS-CONNECT-01 跟踪）。
     *
     * 每次 [connect] 调用均自增，测试可断言 connect 被调用了至少一次。
     */
    var connectCallCount = 0

    /**
     * 最近一次 [connect] 调用时传入的 URL（TC-WS-CONNECT-01 跟踪）。
     *
     * 对应 [RoomSocketRequestSpec.url]；测试可校验 token 被正确追加到查询参数。
     */
    var lastConnectedUrl: String? = null

    /**
     * 注入发送异常（T-30016 SM-05 测试用）。
     *
     * 设置后，下一次 [send] 调用将抛出该异常，模拟网络发送失败场景。
     * 测试结束后应手动重置为 `null`。
     */
    var sendThrowable: Throwable? = null

    // ─── IWebSocketClient ─────────────────────────────────────────────────────

    override suspend fun connect(spec: RoomSocketRequestSpec) {
        connectCallCount++
        lastConnectedUrl = spec.url
        _state.value = WebSocketState.Connecting
        _state.value = WebSocketState.Connected
    }

    /**
     * 发送消息。
     * - [WebSocketState.Connected] 且 [sendThrowable] 为 null：追加到 [sentMessages]，返回 `true`
     * - [sendThrowable] 非 null：抛出该异常（模拟发送失败，用于 T-30016 SM-05 测试）
     * - 其他状态：静默丢弃，返回 `false`，不抛异常
     */
    override fun send(message: String): Boolean {
        sendThrowable?.let { throw it }
        if (_state.value !is WebSocketState.Connected) return false
        sentMessages.add(message)
        return true
    }

    override fun disconnect() {
        _state.value = WebSocketState.Disconnected("manual")
    }

    // ─── 测试辅助方法 ──────────────────────────────────────────────────────────

    /** 模拟服务端推送文本消息，发射 [WebSocketState.Message]。 */
    fun simulateMessage(text: String) {
        _state.value = WebSocketState.Message(text)
    }

    /**
     * 模拟非主动断开，发射 [WebSocketState.Disconnected]。
     * @param reason 断开原因，默认为空字符串
     */
    fun simulateDisconnect(reason: String = "") {
        _state.value = WebSocketState.Disconnected(reason)
    }

    /**
     * 模拟不可恢复错误，发射 [WebSocketState.Error]。
     * @param throwable 导致错误的异常
     */
    fun simulateError(throwable: Throwable) {
        _state.value = WebSocketState.Error(throwable)
    }

    /**
     * 直接将状态设置为 [WebSocketState.Connected]，无需真实握手。
     *
     * 用于 ViewModel 单元测试：跳过 [connect] suspend 函数，直接让
     * [send] 方法正常工作（[send] 会检查 Connected 状态）。
     */
    fun simulateConnect() {
        _state.value = WebSocketState.Connected
    }
}
