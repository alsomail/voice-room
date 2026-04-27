/**
 * 测试套件：WS 网关（API）
 * 用例来源：doc/tests/cases/API/TC-WS.md
 */
import { test, expect } from '@playwright/test';
import WebSocket from 'ws';
import { execSync } from 'child_process';

const WS = process.env.APP_WS_URL!;
const ADMIN = process.env.ADMIN_SERVER_BASE_URL!;
const T = process.env.E2E_VALID_TOKEN ?? '';
const EXP = process.env.E2E_EXPIRED_TOKEN ?? '';
const OP = process.env.E2E_OP_TOKEN ?? '';
const UID = process.env.E2E_USER_A_ID ?? '';
const redis = (s: string) => execSync(`redis-cli ${s}`, { encoding: 'utf-8' }).trim();

test.describe('TC-WS API - WebSocket 网关', () => {
  test('TC-WS-00001: 握手 JWT 正确/错误', async () => {
    test.skip(!T, '需要 Token');
    await new Promise<void>((ok, ko) => {
      const w = new WebSocket(`${WS}?token=${T}`);
      w.once('open', () => { w.close(); ok(); });
      w.once('error', ko);
    });
    await new Promise<void>((ok) => {
      const w = new WebSocket(`${WS}?token=invalid`);
      w.once('close', (code) => { expect([1002, 1008, 4001]).toContain(code); ok(); });
      w.once('error', () => {});
    });
  });

  test('TC-WS-00002: 30s 无心跳断开', async () => {
    test.skip(process.env.CI_E2E_READY !== '1', '耗时用例');
    test.setTimeout(60_000);
    const w = new WebSocket(`${WS}?token=${T}`);
    await new Promise<void>((r) => w.once('open', () => r()));
    const closed = await new Promise<boolean>((ok) => {
      w.once('close', () => ok(true));
      setTimeout(() => ok(false), 45_000);
    });
    expect(closed).toBe(true);
  });

  test('TC-WS-00003: 断线重连携带 last_msg_id', async () => {
    test.skip(!T, '需要 Token');
    const w = new WebSocket(`${WS}?token=${T}&last_msg_id=0`);
    await new Promise<void>((r) => w.once('open', () => r()));
    // 断开重连
    w.close();
    const w2 = new WebSocket(`${WS}?token=${T}&last_msg_id=100`);
    await new Promise<void>((ok, ko) => {
      w2.once('open', () => ok());
      w2.once('error', ko);
    });
    w2.close();
  });

  test('TC-WS-00004: 1000 并发连接', async () => {
    test.skip(process.env.CI_E2E_READY !== '1', '压测用例');
    test.setTimeout(60_000);
    const conns: WebSocket[] = [];
    for (let i = 0; i < 1000; i++) conns.push(new WebSocket(`${WS}?token=${T}`));
    const openCount = await Promise.all(conns.map((w) =>
      new Promise<number>((ok) => { w.once('open', () => ok(1)); w.once('error', () => ok(0)); })));
    expect(openCount.reduce((a, b) => a + b, 0)).toBeGreaterThanOrEqual(950);
    conns.forEach((w) => w.close());
  });

  test('TC-WS-00005: 管理员封禁事件推送', async () => {
    test.skip(!T || !OP || !UID, '需要 Token/OP/UID');
    const w = new WebSocket(`${WS}?token=${T}`);
    await new Promise<void>((r) => w.once('open', () => r()));
    const wait = new Promise<any>((ok) => {
      w.on('message', (d) => { const m = JSON.parse(d.toString()); if (m.type === 'UserBanned') ok(m); });
    });
    // 触发封禁
    await fetch(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${OP}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ type: 'temporary', duration_hours: 1, reason: 'ws-test' }),
    });
    const m = await Promise.race([wait, new Promise((_, ko) => setTimeout(() => ko(new Error('t')), 5000))]);
    expect((m as any).type).toBe('UserBanned');
    w.close();
    // 解封复位
    await fetch(`${ADMIN}/api/v1/admin/users/${UID}/unban`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${OP}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: 'restore' }),
    });
  });

  test('TC-WS-00006: 关闭房间广播', async () => {
    test.skip(process.env.CI_E2E_READY !== '1', '需前置开启房间');
    const RID = process.env.E2E_ROOM_ID ?? '';
    const w = new WebSocket(`${WS}?token=${T}`);
    await new Promise<void>((r) => w.once('open', () => r()));
    w.send(JSON.stringify({ type: 'JoinRoom', room_id: RID, msg_id: 'jr' }));
    const p = new Promise<any>((ok) => {
      w.on('message', (d) => { const m = JSON.parse(d.toString()); if (m.type === 'RoomClosed') ok(m); });
    });
    await fetch(`${ADMIN}/api/v1/admin/rooms/${RID}/force-close`, {
      method: 'POST', headers: { Authorization: `Bearer ${OP}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: 'ws-test' }),
    });
    const m = await Promise.race([p, new Promise((_, ko) => setTimeout(() => ko(new Error('t')), 5000))]);
    expect((m as any).type).toBe('RoomClosed');
    w.close();
  });

  test('TC-WS-00007: 事件处理失败不影响主服务', async () => {
    test.skip(process.env.CI_E2E_READY !== '1', '需构造异常事件');
    // 构造异常 payload
    execSync('redis-cli PUBLISH admin.events \'{"broken":true}\'');
    // 主服务仍可连接
    const w = new WebSocket(`${WS}?token=${T}`);
    await new Promise<void>((ok, ko) => { w.once('open', () => { w.close(); ok(); }); w.once('error', ko); });
  });

  test('TC-WS-00008: HyperLogLog 在线人数', async () => {
    test.skip(!T, '需要 Token');
    const w = new WebSocket(`${WS}?token=${T}`);
    await new Promise<void>((r) => w.once('open', () => r()));
    await new Promise((r) => setTimeout(r, 500));
    const n = Number(redis('PFCOUNT online:users'));
    expect(n).toBeGreaterThanOrEqual(1);
    w.close();
  });
});
