package com.voice.room.android.core.ws

import kotlinx.coroutines.flow.StateFlow

/**
 * WebSocket 客户端契约接口。
 *
 * 所有状态变更均通过 [state] StateFlow 广播，调用方仅需订阅该 Flow 即可响应连接
 * 变化、消息到达和错误事件，无需轮询或回调注册。
 *
 * 实现类：[OkHttpWebSocketClient]（生产）、[FakeWebSocketClient]（测试）
 */
interface IWebSocketClient {

    /**
     * 当前连接状态流，初始值为 [WebSocketState.Disconnected]。
     * 线程安全，可从任意线程收集。
     */
    val state: StateFlow<WebSocketState>

    /**
     * 建立 WebSocket 连接。
     *
     * 调用后状态立即切换为 [WebSocketState.Connecting]；连接握手完成后切换为
     * [WebSocketState.Connected]。可多次调用（如手动重连）。
     *
     * @param spec 由 [RoomSocketRequestFactory.create] 生成的连接规格（含 URL 与鉴权头）
     */
    suspend fun connect(spec: RoomSocketRequestSpec)

    /**
     * 发送文本消息。
     *
     * 当前状态为 [WebSocketState.Connected] 时发送；否则静默丢弃，不抛异常。
     *
     * @param message 原始 JSON 文本
     * @return `true` 表示消息已写入发送队列；`false` 表示当前未连接或发送失败
     */
    fun send(message: String): Boolean

    /**
     * 主动断开连接，不触发自动重连。
     *
     * 调用后状态切换为 [WebSocketState.Disconnected]("manual")，
     * 并取消所有心跳与重连协程。
     */
    fun disconnect()
}
