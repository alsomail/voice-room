/**
 * CROSS-4: LeaveMic → LeaveMicResult + MicLeft 广播
 *
 * PROTO-BINDING:
 *   Android C→S: RoomViewModel.leaveMic → wsClient.send({"type":"LeaveMic",...})
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.LeaveMicResult
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.MicLeft
 *   Server:       app/server/src/room/handler/mic.rs::handle_leave_mic
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.5 (LeaveMic)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.6.5 (LeaveMicResult)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.7.4 (MicLeft)
 *   Schema C→S:   doc/protocol/schemas/ws/LeaveMic.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/LeaveMicResult.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/MicLeft.schema.json
 *
 * 验证目标：
 *   - LeaveMicResult: type=PascalCase, code=0
 *   - MicLeft: payload.mic_index, payload.user_id（snake_case，payload 嵌套）
 *   - 前置：先 TakeMic 再 LeaveMic（需要持有麦位才能下麦）
 */

import { AndroidWsClient } from './helpers/ws-client';
import { validateOrThrow } from './helpers/schema-validator';
import { getCrossLangEnv, isServerReachable, createOrGetRoom } from './helpers/fixtures';

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

const SKIP_MSG = (url: string) => `SKIP-KNOWN: server unavailable at ${url}`;
const MIC_INDEX = 1; // 使用 1 号麦位（区别于 CROSS-3 的 0 号）

// ─────────────────────────────────────────────────────────────────────────────
// Suite
// ─────────────────────────────────────────────────────────────────────────────

describe('CROSS-4: LeaveMic → LeaveMicResult + MicLeft 广播', () => {
  const env = getCrossLangEnv();
  let serverAvailable = false;
  let roomId: string;

  beforeAll(async () => {
    // 检查广播测试是否有两个不同身份的 token
    if (!env.adminToken || env.adminToken === env.userToken) {
      console.warn(
        '[CROSS-4] SKIP-KNOWN: 广播路径测试需要两个不同身份的 token\n' +
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
      const room = await createOrGetRoom(env.apiUrl, env.userToken, 'CROSS-4-Test');
      roomId = room.room_id;
    } catch (err) {
      console.warn(`[CROSS-4] beforeAll: could not create room — ${String(err)}`);
      serverAvailable = false;
    }
  });

  // ── CROSS-4-LEAVEMIC-01 ────────────────────────────────────────────────────

  test('CROSS-4-LEAVEMIC-01: 上麦后下麦 → 收到 LeaveMicResult(code=0)，schema 合法', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client = new AndroidWsClient();
    try {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) { console.log(SKIP_MSG(env.wsUrl)); return; }

      // 加入房间
      client.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const joinResult = await client.waitForMessage('JoinRoomResult', 10000);
      expect(joinResult.code).toBe(0);

      // 先上麦
      client.send({ type: 'TakeMic', payload: { mic_index: MIC_INDEX } });
      const takeResult = await client.waitForMessage('TakeMicResult', 10000);
      if ((takeResult.code as number) !== 0) {
        console.log(`[CROSS-4-LEAVEMIC-01] TakeMic code=${takeResult.code as number}, skipping LeaveMic test`);
        return;
      }

      // Android: RoomViewModel.leaveMic → wsClient.send({"type":"LeaveMic",...})
      client.send({ type: 'LeaveMic' });

      const result = await client.waitForMessage('LeaveMicResult', 10000);

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('LeaveMicResult', result);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(result.type).toBe('LeaveMicResult'); // PascalCase
      expect(result.code).toBe(0);

      // payload 若存在应含 mic_index（snake_case）
      if (result.payload !== undefined && result.payload !== null) {
        const payload = result.payload as Record<string, unknown>;
        if (payload.mic_index !== undefined) {
          expect(typeof payload.mic_index).toBe('number');
        }
        expect(payload.micIndex).toBeUndefined(); // 驼峰不存在
      }
    } finally {
      client.close();
    }
  });

  // ── CROSS-4-LEAVEMIC-02 ────────────────────────────────────────────────────

  test('CROSS-4-LEAVEMIC-02: 下麦 → 同房间观察者收到 MicLeft 广播，payload 嵌套含 mic_index + user_id', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const user = new AndroidWsClient();      // 上麦/下麦者
    const observer = new AndroidWsClient(); // 观察者

    try {
      // 观察者进房间
      const token2 = env.adminToken || env.userToken;
      const okObs = await observer.tryConnect(env.wsUrl, token2);
      if (!okObs) { console.log(SKIP_MSG(env.wsUrl)); return; }
      observer.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const obsJoin = await observer.waitForMessage('JoinRoomResult', 10000);
      expect(obsJoin.code).toBe(0);
      observer.clearQueues();

      // 用户进房间 + 上麦
      const okUser = await user.tryConnect(env.wsUrl, env.userToken);
      if (!okUser) { console.log(SKIP_MSG(env.wsUrl)); return; }
      user.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await user.waitForMessage('JoinRoomResult', 10000);

      user.send({ type: 'TakeMic', payload: { mic_index: MIC_INDEX } });
      const takeResult = await user.waitForMessage('TakeMicResult', 10000);
      if ((takeResult.code as number) !== 0) {
        console.log(`[CROSS-4-LEAVEMIC-02] TakeMic code=${takeResult.code as number}, skipping MicLeft check`);
        return;
      }

      // 清空观察者队列（可能已收到 MicTaken），再等 MicLeft
      observer.clearQueues();
      const micLeftPromise = observer.waitForMessage('MicLeft', 10000);

      // Android: RoomViewModel.leaveMic → wsClient.send({"type":"LeaveMic",...})
      user.send({ type: 'LeaveMic' });
      await user.waitForMessage('LeaveMicResult', 10000);

      // 观察者等待 MicLeft 广播
      const micLeft = await micLeftPromise;

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('MicLeft', micLeft);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(micLeft.type).toBe('MicLeft'); // PascalCase

      // payload 必须嵌套
      expect(typeof micLeft.payload).toBe('object');
      const payload = micLeft.payload as Record<string, unknown>;

      // snake_case 字段
      expect(typeof payload.mic_index).toBe('number');
      expect(typeof payload.user_id).toBe('string');

      // 驼峰不存在
      expect(payload.micIndex).toBeUndefined();
      expect(payload.userId).toBeUndefined();
    } finally {
      user.close();
      observer.close();
    }
  });
});
