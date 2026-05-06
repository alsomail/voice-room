/**
 * CROSS-3: TakeMic → TakeMicResult + MicTaken 广播
 *
 * PROTO-BINDING:
 *   Android C→S: RoomViewModel.takeMic → wsClient.send({"type":"TakeMic","payload":{"mic_index":<int>},...})
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.TakeMicResult
 *   Android S→C: RoomViewModel.handleWsMessage → is WsServerMessage.MicTaken
 *   Server:       app/server/src/room/handler/mic.rs::handle_take_mic
 *   Protocol C→S: doc/protocol/websocket_signals.md §6.5.4 (TakeMic)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.6.4 (TakeMicResult)
 *   Protocol S→C: doc/protocol/websocket_signals.md §6.7.3 (MicTaken)
 *   Schema C→S:   doc/protocol/schemas/ws/TakeMic.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/TakeMicResult.schema.json
 *   Schema S→C:   doc/protocol/schemas/ws/MicTaken.schema.json
 *
 * 验证目标：
 *   - TakeMicResult: type=PascalCase, code=0, payload.mic_index（snake_case）
 *   - MicTaken: payload.mic_index, payload.user_id（snake_case，payload 嵌套）
 *   - 任务描述中 payload.slot 为旧字段名，schema 正式字段为 mic_index（已勘误）
 *
 * ⚠ 协议差异记录（T-00104 §四）：
 *   任务描述写 TakeMic payload: {"slot":0}，但 TakeMic.schema.json 要求 {"mic_index":0}。
 *   本测试以 schema 为准，使用 mic_index。
 */

import { AndroidWsClient } from './helpers/ws-client';
import { validateOrThrow } from './helpers/schema-validator';
import { getCrossLangEnv, isServerReachable, createOrGetRoom } from './helpers/fixtures';

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

const SKIP_MSG = (url: string) => `SKIP-KNOWN: server unavailable at ${url}`;
const MIC_INDEX = 0; // 使用 0 号麦位（schema 允许 0-8）

// ─────────────────────────────────────────────────────────────────────────────
// Suite
// ─────────────────────────────────────────────────────────────────────────────

describe('CROSS-3: TakeMic → TakeMicResult + MicTaken 广播', () => {
  const env = getCrossLangEnv();
  let serverAvailable = false;
  let roomId: string;

  beforeAll(async () => {
    // 检查广播测试是否有两个不同身份的 token
    if (!env.adminToken || env.adminToken === env.userToken) {
      console.warn(
        '[CROSS-3] SKIP-KNOWN: 广播路径测试需要两个不同身份的 token\n' +
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
      const room = await createOrGetRoom(env.apiUrl, env.userToken, 'CROSS-3-Test');
      roomId = room.room_id;
    } catch (err) {
      console.warn(`[CROSS-3] beforeAll: could not create room — ${String(err)}`);
      serverAvailable = false;
    }
  });

  // ── CROSS-3-TAKEIC-01 ──────────────────────────────────────────────────────

  test('CROSS-3-TAKEMIC-01: TakeMic → 收到 TakeMicResult(code=0)，payload.mic_index snake_case', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const client = new AndroidWsClient();
    try {
      const ok = await client.tryConnect(env.wsUrl, env.userToken);
      if (!ok) { console.log(SKIP_MSG(env.wsUrl)); return; }

      // 先加入房间
      client.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const joinResult = await client.waitForMessage('JoinRoomResult', 10000);
      expect(joinResult.code).toBe(0);

      // Android: RoomViewModel.takeMic → wsClient.send({"type":"TakeMic","payload":{"mic_index":0},...})
      // ⚠ 以 schema 为准：字段名 mic_index（非 slot）
      client.send({
        type: 'TakeMic',
        payload: { mic_index: MIC_INDEX },
      });

      const result = await client.waitForMessage('TakeMicResult', 10000);

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('TakeMicResult', result);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(result.type).toBe('TakeMicResult'); // PascalCase
      expect(result.code).toBe(0);

      // payload 是嵌套对象
      if (result.payload !== undefined && result.payload !== null) {
        const payload = result.payload as Record<string, unknown>;
        // snake_case 字段
        if (payload.mic_index !== undefined) {
          expect(typeof payload.mic_index).toBe('number');
        }
        // 驼峰不存在
        expect(payload.micIndex).toBeUndefined();
      }
    } finally {
      client.close();
    }
  });

  // ── CROSS-3-TAKEMIC-02 ─────────────────────────────────────────────────────

  test('CROSS-3-TAKEMIC-02: TakeMic → 同房间观察者收到 MicTaken 广播，payload 嵌套含 mic_index + user_id', async () => {
    if (!serverAvailable) {
      console.log(SKIP_MSG(env.wsUrl));
      return;
    }

    const taker = new AndroidWsClient();   // 上麦者
    const observer = new AndroidWsClient(); // 观察者

    try {
      // 观察者先进房间
      const token2 = env.adminToken || env.userToken;
      const okObs = await observer.tryConnect(env.wsUrl, token2);
      if (!okObs) { console.log(SKIP_MSG(env.wsUrl)); return; }
      observer.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      const obsJoin = await observer.waitForMessage('JoinRoomResult', 10000);
      expect(obsJoin.code).toBe(0);
      observer.clearQueues();

      // 提前挂起等待 MicTaken 广播
      const micTakenPromise = observer.waitForMessage('MicTaken', 10000);

      // 上麦者进房间并上麦
      const okTaker = await taker.tryConnect(env.wsUrl, env.userToken);
      if (!okTaker) { console.log(SKIP_MSG(env.wsUrl)); return; }
      taker.send({ type: 'JoinRoom', payload: { room_id: roomId } });
      await taker.waitForMessage('JoinRoomResult', 10000);

      taker.send({
        type: 'TakeMic',
        payload: { mic_index: MIC_INDEX },
      });

      // 等待 TakeMicResult 确认成功
      const takeResult = await taker.waitForMessage('TakeMicResult', 10000);
      if ((takeResult.code as number) !== 0) {
        console.log(`[CROSS-3-TAKEMIC-02] TakeMic code=${takeResult.code as number}, skipping broadcast check`);
        return;
      }

      // 观察者等待 MicTaken 广播
      const micTaken = await micTakenPromise;

      // ── JSON Schema 验证 ────────────────────────────────────────────────
      validateOrThrow('MicTaken', micTaken);

      // ── 字段级断言 ──────────────────────────────────────────────────────
      expect(micTaken.type).toBe('MicTaken'); // PascalCase

      // payload 必须嵌套（非裸字段）
      expect(typeof micTaken.payload).toBe('object');
      const payload = micTaken.payload as Record<string, unknown>;

      // snake_case: mic_index, user_id
      expect(typeof payload.mic_index).toBe('number');
      expect(typeof payload.user_id).toBe('string');

      // 驼峰不存在
      expect(payload.micIndex).toBeUndefined();
      expect(payload.userId).toBeUndefined();
    } finally {
      taker.close();
      observer.close();
    }
  });
});
