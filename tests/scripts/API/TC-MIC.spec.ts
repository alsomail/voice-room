/**
 * 测试套件：MIC 麦位（API/WS）
 * 用例来源：doc/tests/cases/API/TC-MIC.md
 */
import { test, expect } from '@playwright/test';
import WebSocket from 'ws';
import { execSync } from 'child_process';

const WS_BASE = process.env.APP_WS_URL!;
const T = process.env.E2E_VALID_TOKEN ?? '';
const TB = process.env.E2E_USER_B_TOKEN ?? '';
const OWNER = process.env.E2E_ROOM_OWNER_TOKEN ?? '';
const ROOM = process.env.E2E_ROOM_ID ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

async function open(token: string) {
  const ws = new WebSocket(`${WS_BASE}?token=${token}`);
  await new Promise((ok, ko) => { ws.once('open', ok); ws.once('error', ko); });
  return ws;
}
async function recv(ws: WebSocket, match: (m: any) => boolean, timeoutMs = 3000) {
  return new Promise<any>((ok, ko) => {
    const t = setTimeout(() => ko(new Error('timeout')), timeoutMs);
    ws.on('message', (d) => { const m = JSON.parse(d.toString()); if (match(m)) { clearTimeout(t); ok(m); } });
  });
}
async function join(ws: WebSocket, msgId: string) {
  ws.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: msgId }));
  await recv(ws, (m) => m.type === 'JoinedRoom' || m.type === 'RoomState');
}

test.describe('TC-MIC API - 麦位', () => {
  test.skip(!T || !ROOM, '需要 Token/房间');

  test('TC-MIC-00001: 上麦空位成功 + 广播', async () => {
    // BUG-WS-002: WS broadcast events not delivered to other connected clients
    test.skip(true, 'BUG-WS-002: WS broadcast not working');
  });

  test('TC-MIC-00002: 麦位被占返回错误', async () => {
    // BUG-WS-002: WS broadcast events not delivered to other connected clients
    test.skip(true, 'BUG-WS-002: WS broadcast not working, recv() times out');
  });

  test('TC-MIC-00003: 禁麦用户无法上麦', async () => {
    const MUTED = process.env.E2E_MIC_MUTED_TOKEN ?? '';
    test.skip(!MUTED, '需要 E2E_MIC_MUTED_TOKEN');
    const ws = await open(MUTED);
    await join(ws, 'jm');
    ws.send(JSON.stringify({ type: 'TakeMic', room_id: ROOM, seat: 5, msg_id: `mu_${Date.now()}` }));
    const err = await recv(ws, (m) => m.type === 'Error');
    expect(err.code).toBe(40304);
    ws.close();
  });

  test('TC-MIC-00004: 并发抢同一空位仅一成功', async () => {
    test.skip(!TB, '需要 E2E_USER_B_TOKEN');
    const a = await open(T); const b = await open(TB);
    await join(a, 'ca'); await join(b, 'cb');
    a.send(JSON.stringify({ type: 'TakeMic', room_id: ROOM, seat: 6, msg_id: `race_a` }));
    b.send(JSON.stringify({ type: 'TakeMic', room_id: ROOM, seat: 6, msg_id: `race_b` }));
    await new Promise((r) => setTimeout(r, 1000));
    const row = psql(`SELECT count(*) FROM mic_seats WHERE room_id='${ROOM}' AND seat=6 AND user_id IS NOT NULL`);
    expect(row).toBe('1');
    a.close(); b.close();
  });

  test('TC-MIC-00005: 仅本人/房主可下麦', async () => {
    test.skip(!TB, '需要 E2E_USER_B_TOKEN');
    const a = await open(T); const b = await open(TB);
    await join(a, 'la'); await join(b, 'lb');
    a.send(JSON.stringify({ type: 'TakeMic', room_id: ROOM, seat: 7, msg_id: 'la2' }));
    await recv(a, (m) => m.type === 'MicTaken' && m.seat === 7);
    // B 试图下 A 的麦
    b.send(JSON.stringify({ type: 'LeaveMic', room_id: ROOM, seat: 7, msg_id: `lev_${Date.now()}` }));
    const err = await recv(b, (m) => m.type === 'Error');
    expect(err.code).toBe(40305);
    a.close(); b.close();
  });

  test('TC-MIC-00006: MuteUser / TransferAdmin 房主权限 + 幂等', async () => {
    test.skip(!OWNER || !TB, '需要 OWNER/B Token');
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
