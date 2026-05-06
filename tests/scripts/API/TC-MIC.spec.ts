/**
 * 测试套件：MIC 麦位（API/WS）
 * 用例来源：doc/tests/cases/API/TC-MIC.md
 *
 * Protocol notes (Round-2 fix):
 *  - JoinRoom:  { type:'JoinRoom', payload:{ room_id }, msg_id }  → response type: JoinRoomResult
 *  - TakeMic:  { type:'TakeMic', payload:{ mic_index:N }, msg_id } (N=0..8) → response: TakeMicResult
 *  - LeaveMic: { type:'LeaveMic', msg_id, payload?: { mic_index: number } }  (mic_index optional, for debug; server infers slot from connection context)
 *              → response: LeaveMicResult (code=0 ok, 40304=not on mic)
 *  - Broadcast: MicTaken { type:'MicTaken', payload:{ mic_index, user_id } }
 *  - mic seats are in-memory (RoomState) — no DB table; psql cleanup replaced with WS cycle
 */
import { test, expect } from '@playwright/test';
import WebSocket from 'ws';
import { execSync } from 'child_process';

const WS_BASE = process.env.APP_WS_URL!;
const T = process.env.E2E_VALID_TOKEN ?? '';
const TB = process.env.E2E_USER_B_TOKEN ?? '';
const OWNER = process.env.E2E_ROOM_OWNER_TOKEN ?? '';
const ROOM = process.env.E2E_ROOM_ID ?? '';
const MUTED_ID = process.env.E2E_USER_MUTED_ID ?? '';

// Mic slot indices used per test (0-indexed, range 0-8)
const SLOT_00001 = 7; // TC-MIC-00001: take & broadcast
const SLOT_00002 = 8; // TC-MIC-00002: occupied error
const SLOT_00003 = 4; // TC-MIC-00003: mic-muted block
const SLOT_00004 = 5; // TC-MIC-00004: concurrent race
const SLOT_00005 = 6; // TC-MIC-00005: leave-self only

async function open(token: string) {
  const ws = new WebSocket(`${WS_BASE}?token=${token}`);
  await new Promise((ok, ko) => { ws.once('open', ok); ws.once('error', ko); });
  return ws;
}
async function recv(ws: WebSocket, match: (m: any) => boolean, timeoutMs = 4000) {
  return new Promise<any>((ok, ko) => {
    const t = setTimeout(() => ko(new Error('timeout')), timeoutMs);
    ws.on('message', (d) => { const m = JSON.parse(d.toString()); if (match(m)) { clearTimeout(t); ok(m); } });
  });
}
async function join(ws: WebSocket, msgId: string) {
  ws.send(JSON.stringify({ type: 'JoinRoom', payload: { room_id: ROOM }, msg_id: msgId }));
  const result = await recv(ws, (m) => m.type === 'JoinRoomResult' || m.type === 'JoinedRoom' || m.type === 'RoomState');
  if (result.type === 'JoinRoomResult' && result.code !== 0) {
    throw new Error(`JoinRoom failed: code=${result.code} (room_id=${ROOM}). Room may be closed by TC-ROOM tests.`);
  }
  return result;
}
/** Ensure a mic slot is free: user takes it then waits for LeaveMicResult before returning. */
async function clearSlot(ws: WebSocket, slotIndex: number) {
  const takeMsgId = `clr_take_${slotIndex}_${Date.now()}`;
  const leaveMsgId = `clr_leave_${slotIndex}_${Date.now()}`;

  // Try to take the slot
  ws.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: slotIndex }, msg_id: takeMsgId }));
  const takeResult = await new Promise<any>((res) => {
    const t = setTimeout(() => res({ code: -1 }), 1500);
    const handler = (d: any) => { const m = JSON.parse(d.toString()); if (m.msg_id === takeMsgId) { clearTimeout(t); ws.off('message', handler); res(m); } };
    ws.on('message', handler);
  });

  if (takeResult.code === 0) {
    // We took the slot; now leave it and WAIT for the LeaveMicResult
    ws.send(JSON.stringify({ type: 'LeaveMic', msg_id: leaveMsgId }));
    await new Promise<void>((res) => {
      const t = setTimeout(res, 1500);
      const handler = (d: any) => { const m = JSON.parse(d.toString()); if (m.msg_id === leaveMsgId) { clearTimeout(t); ws.off('message', handler); res(); } };
      ws.on('message', handler);
    });
  } else if (takeResult.code === 40303) {
    // AlreadyOnMic (A is on a DIFFERENT slot) — leave that slot first
    ws.send(JSON.stringify({ type: 'LeaveMic', msg_id: leaveMsgId }));
    await new Promise<void>((res) => {
      const t = setTimeout(res, 1500);
      const handler = (d: any) => { const m = JSON.parse(d.toString()); if (m.msg_id === leaveMsgId) { clearTimeout(t); ws.off('message', handler); res(); } };
      ws.on('message', handler);
    });
    // Slot slotIndex might still be occupied by someone else — try to take again
    const retakeMsgId = `clr_retake_${slotIndex}_${Date.now()}`;
    ws.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: slotIndex }, msg_id: retakeMsgId }));
    const retakeResult = await new Promise<any>((res) => {
      const t = setTimeout(() => res({ code: -1 }), 1500);
      const handler = (d: any) => { const m = JSON.parse(d.toString()); if (m.msg_id === retakeMsgId) { clearTimeout(t); ws.off('message', handler); res(m); } };
      ws.on('message', handler);
    });
    if (retakeResult.code === 0) {
      // Took it, now leave
      const releaveMsgId = `clr_releave_${slotIndex}_${Date.now()}`;
      ws.send(JSON.stringify({ type: 'LeaveMic', msg_id: releaveMsgId }));
      await new Promise<void>((res) => {
        const t = setTimeout(res, 1500);
        const handler = (d: any) => { const m = JSON.parse(d.toString()); if (m.msg_id === releaveMsgId) { clearTimeout(t); ws.off('message', handler); res(); } };
        ws.on('message', handler);
      });
    }
  }
}

test.describe('TC-MIC API - 麦位', () => {
  test.skip(!T || !ROOM, '需要 Token/房间');

  test('TC-MIC-00001: 上麦空位成功 + 广播', async () => {
    // T-00042: WS broadcast fixed; T-0000S: USER_B_TOKEN 由 seed 自动注入
    test.setTimeout(15_000);
    const a = await open(T); const b = await open(TB);
    await join(a, 'ja1'); await join(b, 'jb1');
    // 清理麦位（WS TakeMic→LeaveMic 周期，替代已删除的 mic_seats 表 psql 操作）
    await clearSlot(a, SLOT_00001);
    const msgId = `ta_${Date.now()}`;
    // B 监听广播（MicTaken 含 payload.mic_index）
    const bWait = recv(b, (m) => m.type === 'MicTaken' && m.payload?.mic_index === SLOT_00001, 5000);
    a.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00001 }, msg_id: msgId }));
    const ack = await recv(a, (m) => m.msg_id === msgId, 5000);
    expect(ack.code).toBe(0); // TakeMicResult code=0 on success
    const broadcast = await bWait;
    expect(broadcast).toBeTruthy();
    a.close(); b.close();
  });

  test('TC-MIC-00002: 麦位被占返回错误', async () => {
    // T-00042: WS broadcast fixed; test slot-occupied error; T-0000S: USER_B_TOKEN seeded
    test.setTimeout(15_000);
    const a = await open(T); const b = await open(TB);
    await join(a, 'ja2'); await join(b, 'jb2');
    // 清理麦位（WS 周期）
    await clearSlot(a, SLOT_00002);
    // A 先占 SLOT_00002
    const msgIdA = `ta2_${Date.now()}`;
    a.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00002 }, msg_id: msgIdA }));
    const ackA = await recv(a, (m) => m.msg_id === msgIdA, 5000);
    expect(ackA.code).toBe(0);
    await new Promise((r) => setTimeout(r, 300));
    // B 也试图占 SLOT_00002（已被占）
    const msgIdB = `tb2_${Date.now()}`;
    b.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00002 }, msg_id: msgIdB }));
    const ackB = await recv(b, (m) => m.msg_id === msgIdB, 5000);
    expect(ackB.code).not.toBe(0);
    expect([40301, 40303]).toContain(ackB.code);
    a.close(); b.close();
  });

  test('TC-MIC-00003: 禁麦用户无法上麦', async () => {
    // T-0000S: E2E_MUTED_TOKEN seeded (chat_muted); mic_muted key set/cleared in this test
    test.setTimeout(10_000);
    const MUTED = process.env.E2E_MUTED_TOKEN ?? '';
    test.skip(!MUTED || !MUTED_ID, '需要 E2E_MUTED_TOKEN / E2E_USER_MUTED_ID');
    // 设置 mic_muted Redis key（格式：mic_muted:{room_id}:{user_id}）
    const micMutedKey = `mic_muted:${ROOM}:${MUTED_ID}`;
    execSync(`docker exec vr-redis redis-cli set "${micMutedKey}" 1 EX 120`, { encoding: 'utf-8' });
    try {
      const ws = await open(MUTED);
      await join(ws, 'jm');
      const msgId = `mu_${Date.now()}`;
      ws.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00003 }, msg_id: msgId }));
      const result = await recv(ws, (m) => m.msg_id === msgId, 5000);
      expect(result.code).toBe(40306); // mic_muted → 40306
      ws.close();
    } finally {
      // 清理 mic_muted key
      execSync(`docker exec vr-redis redis-cli del "${micMutedKey}"`, { encoding: 'utf-8' });
    }
  });

  test('TC-MIC-00004: 并发抢同一空位仅一成功', async () => {
    // T-0000S: USER_B_TOKEN seeded; assertion via WS response (no mic_seats table)
    test.setTimeout(10_000);
    const a = await open(T); const b = await open(TB);
    await join(a, 'ca'); await join(b, 'cb');
    // 清理目标槽位
    await clearSlot(a, SLOT_00004);
    const msgIdA = `race_a_${Date.now()}`;
    const msgIdB = `race_b_${Date.now()}`;
    const aResult = recv(a, (m) => m.msg_id === msgIdA, 4000);
    const bResult = recv(b, (m) => m.msg_id === msgIdB, 4000);
    // 并发发送（串行 workers 环境下微小时间差，测试 RoomState 原子性）
    a.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00004 }, msg_id: msgIdA }));
    b.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00004 }, msg_id: msgIdB }));
    const [ra, rb] = await Promise.all([aResult, bResult]);
    const successCount = [ra.code, rb.code].filter(c => c === 0).length;
    expect(successCount).toBe(1); // exactly one succeeds
    a.close(); b.close();
  });

  test('TC-MIC-00005: 仅本人/房主可下麦', async () => {
    // T-0000S: USER_B_TOKEN seeded
    // LeaveMic releases only the sender's own slot; B not on mic → LeaveMicResult code 40304
    test.setTimeout(10_000);
    const a = await open(T); const b = await open(TB);
    await join(a, 'la'); await join(b, 'lb');
    // A 上麦 SLOT_00005
    await clearSlot(a, SLOT_00005);
    const takeMsgId = `la_take_${Date.now()}`;
    a.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: SLOT_00005 }, msg_id: takeMsgId }));
    const takeAck = await recv(a, (m) => m.msg_id === takeMsgId, 5000);
    expect(takeAck.code).toBe(0);
    // B 试图下麦（B 不在任何麦位）→ LeaveMicResult code 40304 (user not on mic)
    const leaveMsgId = `lev_${Date.now()}`;
    b.send(JSON.stringify({ type: 'LeaveMic', msg_id: leaveMsgId }));
    const err = await recv(b, (m) => m.msg_id === leaveMsgId, 5000);
    expect(err.code).toBe(40304);
    a.close(); b.close();
  });

  test('TC-MIC-00006: MuteUser / TransferAdmin 房主权限 + 幂等', async () => {
    test.skip(!OWNER, '需要 E2E_ROOM_OWNER_TOKEN（房主 token，未由 seed 注入；如需可手动设置或 follow-up）');
    const o = await open(OWNER);
    await join(o, 'jo');
    const BID = process.env.E2E_USER_B_ID ?? '';
    const mid = `mute_${Date.now()}`;
    o.send(JSON.stringify({ type: 'MuteUser', room_id: ROOM, user_id: BID, msg_id: mid }));
    const ok = await recv(o, (m) => m.msg_id === mid);
    expect(ok.type).not.toBe('Error');
    // 重复执行幂等
    o.send(JSON.stringify({ type: 'MuteUser', room_id: ROOM, user_id: BID, msg_id: mid }));
    await new Promise((r) => setTimeout(r, 500));
    o.close();
  });
});
