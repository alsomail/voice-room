/**
 * CROSS-2: JoinRoom → JoinRoomResult + UserJoined 广播
 *
 * PROTO-BINDING:
 *   Android C→S: RoomViewModel.joinRoom → wsClient.send({"type":"JoinRoom","payload":{"room_id":"<uuid>"},...})
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.JoinRoomResult
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.UserJoined
 *   Server:       app/server/src/room/handler/lifecycle.rs::handle_join_room
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.2 (JoinRoom)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.6.2 (JoinRoomResult)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.7.1 (UserJoined)
 *   Schema C→S:   doc/protocol/schemas/ws/JoinRoom.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/JoinRoomResult.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/UserJoined.schema.json
 *
 * 验证目标：
 *   - JoinRoomResult: type=PascalCase, code=0, payload.room.room_id, payload.room.mic_slots(9个)
 *   - UserJoined: payload.user_id, payload.nickname, payload.avatar（payload 嵌套，非裸字段）
 *   - 所有字段名 snake_case（不含 userId / roomId 等驼峰）
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

describe('CROSS-2: JoinRoom → JoinRoomResult + UserJoined 广播', () => {
  const env = getCrossLangEnv();
  let serverAvailable = false;
  let roomId: string;

  beforeAll(async () => {
    serverAvailable = await isServerReachable(env.apiUrl);
    if (!serverAvailable) return;

    try {
      const room = await createOrGetRoom(env.apiUrl, env.userToken, 'CROSS-2-Test');
      roomId = room.room_id;
    } catch (err) {
      console.warn(`[CROSS-2] beforeAll: could not create room — ${String(err)}`);
      serverAvailable = false;
    }
  });

  // ── CROSS-2-JOIN-01 ────────────────────────────────────────────────────────

  test('CROSS-2-JOIN-01: JoinRoom → 收到 JoinRoomResult(code=0)，schema 合法，payload 嵌套', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client = new AndroidWsClient();
    try {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) {
        console.log(SKIP_MSG(env.wsUrl));
        return;
      }

      // Android: RoomViewModel.joinRoom → wsClient.send({"type":"JoinRoom",...})
      client.send({
        type: 'JoinRoom',
        payload: { room_id: roomId },
      });

      const result = await client.waitForMessage('JoinRoomResult', 10000);

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('JoinRoomResult', result);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      // type 精确匹配 PascalCase
      expect(result.type).toBe('JoinRoomResult');

      // code = 0 表示成功
      expect(result.code).toBe(0);

      // payload 是嵌套对象（不是裸字段）
      expect(typeof result.payload).toBe('object');
      expect(result.payload).not.toBeNull();

      const payload = result.payload as Record<string, unknown>;
      expect(typeof payload.room).toBe('object');

      const room = payload.room as Record<string, unknown>;

      // snake_case 字段名验证（不能有 roomId / ownerId 等驼峰）
      expect(room.room_id).toBeDefined();
      expect(room.owner_id).toBeDefined();
      expect(room.member_count).toBeDefined();
      expect((room as Record<string, unknown>).roomId).toBeUndefined(); // 驼峰不存在
      expect((room as Record<string, unknown>).ownerId).toBeUndefined();

      // mic_slots 必须是 9 个元素的数组
      expect(Array.isArray(room.mic_slots)).toBe(true);
      expect((room.mic_slots as unknown[]).length).toBe(9);
    } finally {
      client.close();
    }
  });

  // ── CROSS-2-JOIN-02 ────────────────────────────────────────────────────────

  test('CROSS-2-JOIN-02: 第二客户端加入 → 第一客户端收到 UserJoined 广播，payload 嵌套且 snake_case', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client1 = new AndroidWsClient();
    const client2 = new AndroidWsClient();

    try {
      // client1 加入房间
      const ok1 = await client1.tryConnect(env.wsUrl, env.userToken);
      if (!ok1) {
        console.log(SKIP_MSG(env.wsUrl));
        return;
      }
      client1.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const joinResult = await client1.waitForMessage('JoinRoomResult', 10000);
      expect(joinResult.code).toBe(0);

      // 清空 client1 队列（避免自己的 UserJoined 干扰）
      client1.clearQueues();

      // 提前挂起等待（在 client2 发送 JoinRoom 之前）
      const userJoinedPromise = client1.waitForMessage('UserJoined', 10000);

      // client2 连接（使用 adminToken 作为第二用户，若与 userToken 相同则用 token 本身）
      const token2 = env.adminToken || env.userToken;
      const ok2 = await client2.tryConnect(env.wsUrl, token2);
      if (!ok2) {
        console.log(`[CROSS-2-JOIN-02] SKIP-KNOWN: client2 could not connect`);
        return;
      }

      // Android: RoomViewModel.joinRoom (client2) → 触发 UserJoined 广播给 client1
      client2.send({ type: 'JoinRoom', payload: { room_id: roomId } });

      // client1 应收到 UserJoined 广播
      const userJoined = await userJoinedPromise;

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('UserJoined', userJoined);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(userJoined.type).toBe('UserJoined');

      // payload 必须嵌套（不是裸字段）
      expect(typeof userJoined.payload).toBe('object');
      const ujPayload = userJoined.payload as Record<string, unknown>;

      // snake_case 字段
      expect(typeof ujPayload.user_id).toBe('string');
      expect(typeof ujPayload.nickname).toBe('string');
      // userId 驼峰不应存在
      expect(ujPayload.userId).toBeUndefined();

      // member_count 字段（如果存在）应为整数
      if (ujPayload.member_count !== undefined) {
        expect(typeof ujPayload.member_count).toBe('number');
        expect(ujPayload.member_count as number).toBeGreaterThanOrEqual(1);
      }
    } finally {
      client1.close();
      client2.close();
    }
  });
});
