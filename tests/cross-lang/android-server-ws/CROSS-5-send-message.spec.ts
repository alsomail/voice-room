/**
 * CROSS-5: SendMessage → RoomMessage 广播
 *
 * PROTO-BINDING:
 *   Android C→S: RoomViewModel.sendMessage → wsClient.send({"type":"SendMessage","payload":{"content":"<text>"},...})
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.RoomMessage
 *   Server:       app/server/src/room/handler/chat.rs::handle_send_message
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.6 (SendMessage)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.7.5 (RoomMessage)
 *   Schema C→S:   doc/protocol/schemas/ws/SendMessage.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/RoomMessage.schema.json
 *
 * 验证目标：
 *   - RoomMessage: payload.content 内容匹配发送内容
 *   - RoomMessage: payload.user_id, payload.msg_id（snake_case，payload 嵌套）
 *   - 特殊字符（中文、emoji、Unicode）payload.content 不被截断
 */

import { AndroidWsClient } from './helpers/ws-client';
import { validateOrThrow } from './helpers/schema-validator';
import { getCrossLangEnv, isServerReachable, createOrGetRoom } from './helpers/fixtures';

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

const SKIP_MSG = (url: string) => `SKIP-KNOWN: server unavailable at ${url}`;

// ─────────────────────────────────────────────────────────────────────────────
// Suite
// ─────────────────────────────────────────────────────────────────────────────

describe('CROSS-5: SendMessage → RoomMessage 广播', () => {
  const env = getCrossLangEnv();
  let serverAvailable = false;
  let roomId: string;

  beforeAll(async () => {
    // 检查广播测试是否有两个不同身份的 token
    if (!env.adminToken || env.adminToken === env.userToken) {
      console.warn(
        '[CROSS-5] SKIP-KNOWN: 广播路径测试需要两个不同身份的 token\n' +
        '  请在 tests/scripts/env/.env.local 中配置:\n' +
        '  E2E_TOKEN_USER1=<user-jwt>\n' +
        '  E2E_TOKEN_ADMIN=<admin-jwt>  (需与 USER1 为不同用户)\n' +
        '  当前两个 token 相同或缺失，广播场景将走 SKIP-KNOWN 路径',
      );
      serverAvailable = false;
      return;
    }

    serverAvailable = await isServerReachable(env.apiUrl);
    if (!serverAvailable) return;

    try {
      const room = await createOrGetRoom(env.apiUrl, env.userToken, 'CROSS-5-Test');
      roomId = room.room_id;
    } catch (err) {
      console.warn(`[CROSS-5] beforeAll: could not create room — ${String(err)}`);
      serverAvailable = false;
    }
  });

  // ── CROSS-5-MSG-01 ─────────────────────────────────────────────────────────

  test('CROSS-5-MSG-01: SendMessage → 同房间收到 RoomMessage，payload.content 内容匹配', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const sender = new AndroidWsClient();
    const receiver = new AndroidWsClient();

    try {
      // receiver 进房间
      const token2 = env.adminToken || env.userToken;
      const okRec = await receiver.tryConnect(env.wsUrl, token2);
      if (!okRec) { console.log(SKIP_MSG(env.wsUrl)); return; }
      receiver.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await receiver.waitForMessage('JoinRoomResult', 10000);
      receiver.clearQueues();

      // receiver 提前挂起等待 RoomMessage
      const roomMsgPromise = receiver.waitForMessage('RoomMessage', 10000);

      // sender 进房间
      const okSender = await sender.tryConnect(env.wsUrl, env.userToken);
      if (!okSender) { console.log(SKIP_MSG(env.wsUrl)); return; }
      sender.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await sender.waitForMessage('JoinRoomResult', 10000);

      const testContent = 'Hello from CROSS-5 test 🎤';

      // Android: RoomViewModel.sendMessage → wsClient.send({"type":"SendMessage","payload":{"content":"..."},...})
      sender.send({
        type: 'SendMessage',
        payload: { content: testContent },
      });

      // receiver 等待 RoomMessage 广播
      const roomMsg = await roomMsgPromise;

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('RoomMessage', roomMsg);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(roomMsg.type).toBe('RoomMessage'); // PascalCase

      // payload 必须嵌套
      expect(typeof roomMsg.payload).toBe('object');
      const payload = roomMsg.payload as Record<string, unknown>;

      // content 字段必须匹配（snake_case）
      expect(payload.content).toBe(testContent);

      // snake_case 字段
      expect(typeof payload.user_id).toBe('string');
      expect(typeof payload.msg_id).toBe('string');

      // 驼峰不存在
      expect(payload.userId).toBeUndefined();
      expect(payload.msgId).toBeUndefined();
    } finally {
      sender.close();
      receiver.close();
    }
  });

  // ── CROSS-5-MSG-02 ─────────────────────────────────────────────────────────

  test('CROSS-5-MSG-02: SendMessage 含中文 + emoji → RoomMessage payload.content 不被截断', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client = new AndroidWsClient();
    try {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) { console.log(SKIP_MSG(env.wsUrl)); return; }

      client.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await client.waitForMessage('JoinRoomResult', 10000);
      client.clearQueues();

      // 特殊字符测试用例
      const specialContent = '你好世界 🌏 مرحبا test-123';

      client.send({
        type: 'SendMessage',
        payload: { content: specialContent },
      });

      // 发送者自己也应该收到 RoomMessage 广播（server 广播给所有人包括发送者）
      const roomMsg = await client.waitForMessage('RoomMessage', 10000);

      // Schema 验证
      validateOrThrow('RoomMessage', roomMsg);

      expect(roomMsg.type).toBe('RoomMessage');
      const payload = roomMsg.payload as Record<string, unknown>;
      expect(payload.content).toBe(specialContent);
    } finally {
      client.close();
    }
  });

  // ── CROSS-5-MSG-03 ─────────────────────────────────────────────────────────

  test('CROSS-5-MSG-03: RoomMessage payload 嵌套验证（不含裸字段 content / user_id）', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client = new AndroidWsClient();
    try {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) { console.log(SKIP_MSG(env.wsUrl)); return; }

      client.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await client.waitForMessage('JoinRoomResult', 10000);
      client.clearQueues();

      client.send({
        type: 'SendMessage',
        payload: { content: 'payload nesting test' },
      });

      const roomMsg = await client.waitForMessage('RoomMessage', 10000);
      validateOrThrow('RoomMessage', roomMsg);

      // 顶级字段中不得含裸 content / user_id（必须在 payload 内）
      expect((roomMsg as Record<string, unknown>).content).toBeUndefined();
      expect((roomMsg as Record<string, unknown>).user_id).toBeUndefined();

      // payload 应是对象
      expect(typeof roomMsg.payload).toBe('object');
    } finally {
      client.close();
    }
  });
});
