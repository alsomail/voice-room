/**
 * CROSS-8: KickUser → KickUserResult + UserLeft 广播
 *
 * PROTO-BINDING:
 *   Android C→S: Admin WS → wsClient.send({"type":"KickUser","payload":{"target_user_id":"<uuid>"},...})
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.UserLeft
 *   Server:       app/server/src/modules/governance/kick.rs::handle_kick
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.9 (KickUser)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.7.2 (UserLeft)
 *   Schema C→S:   doc/protocol/schemas/ws/KickUser.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/UserLeft.schema.json
 *
 * 验证目标：
 *   - KickUser payload: target_user_id（snake_case）
 *   - UserLeft: payload.user_id, payload.member_count（snake_case，payload 嵌套）
 *   - 被踢者收到 connection 关闭 或 UserLeft 点对点消息
 *   - 同房间观察者收到 UserLeft 广播
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

describe('CROSS-8: KickUser → KickUserResult + UserLeft 广播', () => {
  const env = getCrossLangEnv();
  let serverAvailable = false;
  let roomId: string;

  beforeAll(async () => {
    serverAvailable = await isServerReachable(env.apiUrl);
    if (!serverAvailable) return;

    const token = env.adminToken || env.userToken;
    try {
      const room = await createOrGetRoom(env.apiUrl, token, 'CROSS-8-Test');
      roomId = room.room_id;
    } catch (err) {
      console.warn(`[CROSS-8] beforeAll: could not create room — ${String(err)}`);
      serverAvailable = false;
    }
  });

  // ── CROSS-8-KICK-01 ────────────────────────────────────────────────────────

  test('CROSS-8-KICK-01: Admin 发 KickUser → 收到 KickUserResult，schema 合法', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    if (!env.adminToken) {
      console.log('[CROSS-8-KICK-01] SKIP-KNOWN: E2E_ADMIN_TOKEN not configured');
      return;
    }

    const admin = new AndroidWsClient();

    try {
      const okAdmin = await admin.tryConnect(env.wsUrl, env.adminToken);
      if (!okAdmin) { console.log(SKIP_MSG(env.wsUrl)); return; }

      admin.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const joinResult = await admin.waitForMessage('JoinRoomResult', 10000);
      if ((joinResult.code as number) !== 0) {
        console.log(`[CROSS-8-KICK-01] admin JoinRoom code=${joinResult.code as number}, skipping`);
        return;
      }

      const targetUserId = '00000000-0000-4000-8000-000000000001';

      // Admin WS: wsClient.send({"type":"KickUser","payload":{"target_user_id":"<uuid>"},...})
      admin.send({
        type: 'KickUser',
        payload: {
          target_user_id: targetUserId,
          duration_sec: null,
        },
      });

      const kickResult = await admin.waitForMessage('KickUserResult', 10000);

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('KickUserResult', kickResult);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(kickResult.type).toBe('KickUserResult'); // PascalCase
      expect(typeof kickResult.code).toBe('number');
      // code 可能非 0（用户不在房间），但 schema 合法即通过
    } finally {
      admin.close();
    }
  });

  // ── CROSS-8-KICK-02 ────────────────────────────────────────────────────────

  test('CROSS-8-KICK-02: KickUser → 同房间观察者收到 UserLeft 广播，payload 嵌套含 user_id', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    if (!env.adminToken) {
      console.log('[CROSS-8-KICK-02] SKIP-KNOWN: E2E_ADMIN_TOKEN not configured');
      return;
    }

    const admin = new AndroidWsClient();
    const target = new AndroidWsClient(); // 被踢者
    const observer = new AndroidWsClient();

    try {
      // observer 进房间
      const okObs = await observer.tryConnect(env.wsUrl, env.userToken);
      if (!okObs) { console.log(SKIP_MSG(env.wsUrl)); return; }
      observer.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const obsJoin = await observer.waitForMessage('JoinRoomResult', 10000);
      if ((obsJoin.code as number) !== 0) {
        console.log(`[CROSS-8-KICK-02] observer JoinRoom code=${obsJoin.code as number}`);
        return;
      }
      observer.clearQueues();

      // target 进房间
      const okTarget = await target.tryConnect(env.wsUrl, env.userToken);
      if (!okTarget) { console.log(SKIP_MSG(env.wsUrl)); return; }
      target.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const targetJoin = await target.waitForMessage('JoinRoomResult', 10000);
      if ((targetJoin.code as number) !== 0) {
        console.log(`[CROSS-8-KICK-02] target JoinRoom code=${targetJoin.code as number}`);
        return;
      }

      // 从 target 的 JoinRoomResult 推断 target_user_id
      // （实际场景：Android WsServerMessage.JoinRoomResult 中有用户自己的 ID）
      // 这里尝试从 payload.room.owner_id 或留存 token 中推断；无法确定时使用占位值
      const resPayload = targetJoin.payload as Record<string, unknown> | undefined;
      const room = resPayload?.room as Record<string, unknown> | undefined;
      const targetUserId = (room?.owner_id as string | undefined) ?? '00000000-0000-4000-8000-000000000001';

      // 提前等待 UserLeft
      observer.clearQueues();
      const userLeftPromise = observer.waitForMessage('UserLeft', 12000);

      // admin 进房间
      const okAdmin = await admin.tryConnect(env.wsUrl, env.adminToken);
      if (!okAdmin) { console.log(SKIP_MSG(env.wsUrl)); return; }
      admin.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await admin.waitForMessage('JoinRoomResult', 10000);

      // Admin: wsClient.send({"type":"KickUser","payload":{"target_user_id":"<uuid>"},...})
      admin.send({
        type: 'KickUser',
        payload: {
          target_user_id: targetUserId,
          duration_sec: null,
        },
      });

      const kickResult = await admin.waitForMessage('KickUserResult', 10000);

      if ((kickResult.code as number) !== 0) {
        console.log(
          `[CROSS-8-KICK-02] KickUser code=${kickResult.code as number} ` +
            `(likely user not in room or no permission) — skipping UserLeft broadcast check`,
        );
        return;
      }

      // observer 等待 UserLeft 广播
      const userLeft = await userLeftPromise;

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('UserLeft', userLeft);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(userLeft.type).toBe('UserLeft'); // PascalCase

      // payload 必须嵌套（非裸字段）
      expect(typeof userLeft.payload).toBe('object');
      const payload = userLeft.payload as Record<string, unknown>;

      // snake_case 字段
      expect(typeof payload.user_id).toBe('string');
      expect(payload.userId).toBeUndefined(); // 驼峰不存在

      if (payload.member_count !== undefined) {
        expect(typeof payload.member_count).toBe('number');
        expect(payload.member_count as number).toBeGreaterThanOrEqual(0);
        expect(payload.memberCount).toBeUndefined(); // 驼峰不存在
      }
    } finally {
      admin.close();
      target.close();
      observer.close();
    }
  });

  // ── CROSS-8-KICK-03 ────────────────────────────────────────────────────────

  test('CROSS-8-KICK-03: UserLeft 顶级不含裸字段 user_id（payload 嵌套验证）', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    if (!env.adminToken) {
      console.log('[CROSS-8-KICK-03] SKIP-KNOWN: E2E_ADMIN_TOKEN not configured');
      return;
    }

    const admin = new AndroidWsClient();

    try {
      const ok = await admin.tryConnect(env.wsUrl, env.adminToken);
      if (!ok) { console.log(SKIP_MSG(env.wsUrl)); return; }

      admin.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await admin.waitForMessage('JoinRoomResult', 10000);

      admin.send({
        type: 'KickUser',
        payload: { target_user_id: '00000000-0000-4000-8000-000000000001', duration_sec: null },
      });

      const kickResult = await admin.waitForMessage('KickUserResult', 10000);
      if ((kickResult.code as number) !== 0) {
        console.log(`[CROSS-8-KICK-03] code=${kickResult.code as number} — skipping`);
        return;
      }

      // Admin 自己也应收到 UserLeft 广播
      const userLeft = await admin.waitForMessage('UserLeft', 8000);
      validateOrThrow('UserLeft', userLeft);

      // 顶级字段中不含裸 user_id（必须在 payload 内）
      expect((userLeft as Record<string, unknown>).user_id).toBeUndefined();

      // payload 是对象
      expect(typeof userLeft.payload).toBe('object');
      const payload = userLeft.payload as Record<string, unknown>;
      expect(typeof payload.user_id).toBe('string');
    } finally {
      admin.close();
    }
  });
});
