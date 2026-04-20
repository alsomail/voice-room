package com.voice.room.android.core.ws

/**
 * Sealed class representing all possible WebSocket connection states.
 *
 * State flow diagram:
 *
 *   Disconnected ──connect()──► Connecting ──onOpen──► Connected
 *        ▲                          ▲                      │
 *        │                          │         onMessage ──► Message(text)
 *        │                     scheduleReconnect()          │
 *        │                          │               onClosed/onFailure
 *        │                          └──────────────────────┤
 *        └────────── disconnect() / maxRetries ─────────────┘
 *                          Error(throwable)
 */
sealed class WebSocketState {

    /** WebSocket 连接正在建立（含重连进行中） */
    object Connecting : WebSocketState()

    /** WebSocket 已连接就绪，可发送消息 */
    object Connected : WebSocketState()

    /**
     * WebSocket 已断开。
     * @param reason 断开原因（主动断开时为 "manual"，服务端关闭时为关闭原因字符串）
     */
    data class Disconnected(val reason: String = "") : WebSocketState()

    /**
     * 收到服务端推送的文本消息。
     * @param text 原始 JSON 文本
     */
    data class Message(val text: String) : WebSocketState()

    /**
     * 不可恢复错误（含超出最大重试次数 [MaxRetryExceededException]）。
     * @param throwable 导致错误的异常
     */
    data class Error(val throwable: Throwable) : WebSocketState()
}
