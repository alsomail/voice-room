/**
 * AndroidWsClient — 用原生 WebSocket (ws 包) 模拟 Android OkHttpWebSocketClient 行为。
 *
 * PROTO-BINDING:
 *   Android: OkHttpWebSocketClient.startHeartbeat → wsClient.send({"type":"Ping",...})
 *   Android: OkHttpWebSocketClient.onMessage       → 消息分发 / pong 检测
 *   Server:  app/server/src/ws/connection.rs::ping_pong_responses
 *   Protocol: doc/protocol/websocket_signals.md §6.5.1 / §6.6.1
 */

import WebSocket from 'ws';
import { randomUUID } from 'crypto';

// ─────────────────────────────────────────────────────────────────────────────
// 类型定义
// ─────────────────────────────────────────────────────────────────────────────

export type WsMessage = Record<string, unknown>;

interface PendingWaiter {
  resolve: (msg: WsMessage) => void;
  reject: (err: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

// ─────────────────────────────────────────────────────────────────────────────
// AndroidWsClient
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 模拟 Android OkHttpWebSocketClient：
 * - 连接时在 URL 附加 ?token=<JWT>
 * - send() 自动注入 msg_id (UUID v4) + timestamp (ms)（如调用方未提供）
 * - 消息采用生产者-消费者队列分发，保证不丢 race condition 消息
 */
export class AndroidWsClient {
  private ws: WebSocket | null = null;
  private _connected = false;

  /** 按消息 type 分桶的接收队列（消费者尚未就绪时缓存） */
  private messageQueues: Map<string, WsMessage[]> = new Map();
  /** 按消息 type 分桶的等待 Promise（消息尚未到达时挂起） */
  private messageWaiters: Map<string, PendingWaiter[]> = new Map();

  // ── 连接 ───────────────────────────────────────────────────────────────────

  /**
   * 尝试连接 WebSocket 服务器。
   * 连接失败（网络不通 / timeout）返回 false，不抛异常。
   * @param url  ws:// 或 wss:// 地址（不含 token 参数）
   * @param token JWT 令牌
   * @param timeoutMs 连接超时（默认 5 s）
   */
  async tryConnect(url: string, token: string, timeoutMs = 5000): Promise<boolean> {
    const wsUrl = `${url}?token=${encodeURIComponent(token)}`;
    return new Promise<boolean>((resolve) => {
      let settled = false;
      const settle = (success: boolean) => {
        if (settled) return;
        settled = true;
        clearTimeout(connectTimer);
        resolve(success);
      };

      const connectTimer = setTimeout(() => {
        ws.terminate();
        settle(false);
      }, timeoutMs);

      const ws = new WebSocket(wsUrl);

      ws.on('open', () => {
        this._connected = true;
        this.ws = ws;
        settle(true);
      });

      ws.on('message', (data: WebSocket.RawData) => {
        try {
          const msg = JSON.parse(data.toString()) as WsMessage;
          this._dispatch(msg);
        } catch {
          // 忽略非 JSON 消息
        }
      });

      ws.on('error', () => {
        this._connected = false;
        settle(false);
      });

      ws.on('close', () => {
        this._connected = false;
      });
    });
  }

  /** 当前是否处于已连接状态 */
  isConnected(): boolean {
    return this._connected;
  }

  // ── 发送 ───────────────────────────────────────────────────────────────────

  /**
   * 发送消息，模拟 Android wsClient.send()。
   * 自动注入 msg_id（UUID v4）与 timestamp（ms），调用方也可在 message 中覆盖。
   */
  send(message: WsMessage): void {
    if (!this.ws || !this._connected) {
      return;
    }
    const envelope: WsMessage = {
      msg_id: randomUUID(),
      timestamp: Date.now(),
      ...message,
    };
    this.ws.send(JSON.stringify(envelope));
  }

  // ── 接收 ───────────────────────────────────────────────────────────────────

  /**
   * 等待指定 type 的消息。若队列中已有，立即返回；否则挂起直到超时。
   *
   * 采用生产者-消费者模型（FIFO），避免 send-then-wait 的 race condition。
   */
  waitForMessage(type: string, timeoutMs = 8000): Promise<WsMessage> {
    // 先检查队列中是否已有该类型消息
    const queue = this.messageQueues.get(type);
    if (queue && queue.length > 0) {
      const msg = queue.shift()!;
      if (queue.length === 0) this.messageQueues.delete(type);
      return Promise.resolve(msg);
    }

    // 挂起等待
    return new Promise<WsMessage>((resolve, reject) => {
      const timer = setTimeout(() => {
        this._removeWaiter(type, waiter);
        reject(new Error(`Timeout (${timeoutMs}ms) waiting for WS message type="${type}"`));
      }, timeoutMs);

      const waiter: PendingWaiter = { resolve, reject, timer };
      const waiters = this.messageWaiters.get(type) ?? [];
      waiters.push(waiter);
      this.messageWaiters.set(type, waiters);
    });
  }

  /**
   * 并发等待多个 type 的消息，返回按 types 顺序对应的消息数组。
   */
  waitForMessages(types: string[], timeoutMs = 8000): Promise<WsMessage[]> {
    return Promise.all(types.map((t) => this.waitForMessage(t, timeoutMs)));
  }

  /** 清空所有已缓存消息（用于测试隔离，例如在 join 完成后重置队列再等待广播）。 */
  clearQueues(): void {
    this.messageQueues.clear();
  }

  // ── 关闭 ───────────────────────────────────────────────────────────────────

  close(): void {
    this._connected = false;
    this.ws?.close();
    this.ws = null;
    // 拒绝所有待定 waiter
    for (const [, waiters] of this.messageWaiters) {
      for (const w of waiters) {
        clearTimeout(w.timer);
        w.reject(new Error('WebSocket closed'));
      }
    }
    this.messageWaiters.clear();
    this.messageQueues.clear();
  }

  // ── 私有方法 ────────────────────────────────────────────────────────────────

  /** 消息分发：优先唤醒等待的 Promise；无等待者则入队缓存。 */
  private _dispatch(msg: WsMessage): void {
    const type = typeof msg.type === 'string' ? msg.type : '__unknown__';
    const waiters = this.messageWaiters.get(type);
    if (waiters && waiters.length > 0) {
      const waiter = waiters.shift()!;
      if (waiters.length === 0) this.messageWaiters.delete(type);
      clearTimeout(waiter.timer);
      waiter.resolve(msg);
      return;
    }
    // 入队
    const queue = this.messageQueues.get(type) ?? [];
    queue.push(msg);
    this.messageQueues.set(type, queue);
  }

  private _removeWaiter(type: string, waiter: PendingWaiter): void {
    const waiters = this.messageWaiters.get(type);
    if (!waiters) return;
    const idx = waiters.indexOf(waiter);
    if (idx >= 0) waiters.splice(idx, 1);
    if (waiters.length === 0) this.messageWaiters.delete(type);
  }
}
