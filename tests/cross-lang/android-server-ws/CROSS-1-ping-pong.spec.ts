/**
 * CROSS-1: Ping/Pong 心跳往返
 *
 * PROTO-BINDING:
 *   Android C→S: OkHttpWebSocketClient.startHeartbeat → wsClient.send({"type":"Ping",...})
 *   Android S→C: OkHttpWebSocketClient.onMessage → pong 检测
 *   Server:       app/server/src/ws/connection.rs::ping_pong_responses
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.1 (Ping)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.6.1 (Pong)
 *   Schema C→S:   doc/protocol/schemas/ws/Ping.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/Pong.schema.json
 *
 * 验证目标：
 *   - Pong 消息 type === 'Pong'（PascalCase）
 *   - Pong 消息 timestamp 为毫秒级整数（> 2024-01-01 基准）
 *   - Pong 消息 msg_id 回显 Ping 的 msg_id
 *   - 连续 5 次 Ping/Pong 不掉线
 */

import { AndroidWsClient } from './helpers/ws-client';
import { validateOrThrow } from './helpers/schema-validator';
import { getCrossLangEnv, isServerReachable } from './helpers/fixtures';
import { randomUUID } from 'crypto';

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

const MS_2024_01_01 = 1_704_067_200_000; // 毫秒基准：2024-01-01T00:00:00Z
const SKIP_MSG = (url: string) => `SKIP-KNOWN: server unavailable at ${url}`;

// ─────────────────────────────────────────────────────────────────────────────
// Suite
// ─────────────────────────────────────────────────────────────────────────────

describe('CROSS-1: Ping/Pong 心跳往返', () => {
  const env = getCrossLangEnv();
  let client: AndroidWsClient;
  let serverAvailable = false;

  beforeAll(async () => {
    serverAvailable = await isServerReachable(env.apiUrl);
  });

  beforeEach(async () => {
    client = new AndroidWsClient();
    if (serverAvailable) {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) serverAvailable = false;
    }
  });

  afterEach(() => {
    client.close();
  });

  // ── CROSS-1-PING-01 ────────────────────────────────────────────────────────

  test('CROSS-1-PING-01: 发送 Ping 收到 Pong，type=PascalCase，timestamp 为毫秒级整数', async () => {
    if (!serverAvailable || !client.isConnected()) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const pingMsgId = randomUUID();

    // 发送 Ping（Android: OkHttpWebSocketClient.startHeartbeat）
    client.send({ type: 'Ping', msg_id: pingMsgId });

    // 等待 Pong（Android: OkHttpWebSocketClient.onMessage → pong 检测）
    const pong = await client.waitForMessage('Pong', 8000);

    // ── JSON Schema 验证 ────────────────────────────────────────────────────
    validateOrThrow('Pong', pong);

    // ── 字段级断言 ──────────────────────────────────────────────────────────
    // type 必须是 PascalCase 精确匹配
    expect(pong.type).toBe('Pong');

    // timestamp 必须是毫秒级整数
    expect(typeof pong.timestamp).toBe('number');
    expect(Number.isInteger(pong.timestamp)).toBe(true);
    expect(pong.timestamp as number).toBeGreaterThan(MS_2024_01_01);

    // msg_id 必须回显 Ping 的 msg_id
    expect(pong.msg_id).toBe(pingMsgId);

    // Pong 不得有 payload（schema additionalProperties: false）
    expect((pong as Record<string, unknown>).payload).toBeUndefined();
  });

  // ── CROSS-1-PING-02 ────────────────────────────────────────────────────────

  test('CROSS-1-PING-02: 连续 5 次 Ping/Pong 不掉线，每次 Pong schema 合法', async () => {
    if (!serverAvailable || !client.isConnected()) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const ROUNDS = 5;

    for (let i = 0; i < ROUNDS; i++) {
      expect(client.isConnected()).toBe(true);

      const pingMsgId = randomUUID();
      client.send({ type: 'Ping', msg_id: pingMsgId });

      const pong = await client.waitForMessage('Pong', 8000);

      // Schema 验证（每轮）
      validateOrThrow('Pong', pong);

      // 字段断言
      expect(pong.type).toBe('Pong');
      expect(pong.msg_id).toBe(pingMsgId);
      expect(pong.timestamp as number).toBeGreaterThan(MS_2024_01_01);

      // 小间隔，模拟心跳节奏
      await new Promise((r) => setTimeout(r, 100));
    }

    // 所有轮次完成后仍保持连接
    expect(client.isConnected()).toBe(true);
  });

  // ── CROSS-1-PING-03 ────────────────────────────────────────────────────────

  test('CROSS-1-PING-03: Pong 中不含 snake_case 以外的字段（驼峰泄漏检测）', async () => {
    if (!serverAvailable || !client.isConnected()) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    client.send({ type: 'Ping' });
    const pong = await client.waitForMessage('Pong', 8000);

    validateOrThrow('Pong', pong);

    // 检查顶级字段名均为 snake_case（schema 只允许 type / msg_id / timestamp）
    const topLevelKeys = Object.keys(pong);
    const camelCasePattern = /[a-z][A-Z]/; // 驼峰特征
    for (const key of topLevelKeys) {
      expect(camelCasePattern.test(key)).toBe(false); // 不允许驼峰字段
    }
  });
});
