/**
 * 测试套件：WALLET 钱包（API）
 * 用例来源：doc/tests/cases/API/TC-WALLET.md
 */
import { test, expect } from '@playwright/test';
import WebSocket from 'ws';
import { execSync } from 'child_process';

const APP = process.env.APP_SERVER_BASE_URL!;
const ADMIN = process.env.ADMIN_SERVER_BASE_URL!;
const WS = process.env.APP_WS_URL!;
const T = process.env.E2E_VALID_TOKEN ?? '';
const FIN = process.env.E2E_FIN_TOKEN ?? '';
const UID = process.env.E2E_USER_A_ID ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

test.describe('TC-WALLET API - 钱包', () => {
  test('TC-WALLET-00001: GET /wallet/balance', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const r = await request.get(`${APP}/api/v1/wallet/balance`, { headers: { Authorization: `Bearer ${T}` } });
    expect(r.status()).toBe(200);
    const d = (await r.json()).data;
    expect(typeof d.coin_balance).toBe('number');
  });

  test('TC-WALLET-00002: GET /wallet/transactions 分页', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const r = await request.get(`${APP}/api/v1/wallet/transactions?page=1&per_page=20`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body.data.items)).toBe(true);
    expect(body.data).toHaveProperty('total');
  });

  test('TC-WALLET-00003: WS BalanceUpdated 多端推送', async () => {
    test.skip(!T || !FIN || !UID, '需要 Token/FIN/UID');
    const wsA = new WebSocket(`${WS}?token=${T}`);
    const wsB = new WebSocket(`${WS}?token=${T}`);
    await Promise.all([
      new Promise<void>((r) => wsA.once('open', () => r())),
      new Promise<void>((r) => wsB.once('open', () => r())),
    ]);
    const waitFor = (ws: WebSocket) => new Promise<any>((ok) => {
      ws.on('message', (d) => { const m = JSON.parse(d.toString()); if (m.type === 'BalanceUpdated') ok(m); });
    });
    const pA = waitFor(wsA); const pB = waitFor(wsB);
    // 触发 Admin 调整
    const adj = await fetch(`${ADMIN}/api/v1/admin/users/${UID}/balance`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${FIN}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ delta: 100, reason: 'e2e-test' }),
    });
    expect(adj.status).toBe(200);
    const [a, b] = await Promise.all([pA, pB]);
    expect(a.coin_balance).toBe(b.coin_balance);
    wsA.close(); wsB.close();
  });

  test('TC-WALLET-00004: Admin 调整余额 + 事务原子性', async ({ request }) => {
    test.skip(!FIN || !UID, '需要 FIN/UID');
    const before = Number(psql(`SELECT coin_balance FROM users WHERE id='${UID}'`));
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/balance`, {
      headers: { Authorization: `Bearer ${FIN}` },
      data: { delta: -50, reason: 'correction' },
    });
    expect(r.status()).toBe(200);
    const after = Number(psql(`SELECT coin_balance FROM users WHERE id='${UID}'`));
    expect(after).toBe(before - 50);
    const tx = Number(psql(`SELECT count(*) FROM transactions WHERE user_id='${UID}' AND delta=-50 ORDER BY created_at DESC LIMIT 1`));
    expect(tx).toBeGreaterThanOrEqual(1);
  });

  test('TC-WALLET-00005: 事务失败回滚', async ({ request }) => {
    test.skip(!FIN || !UID, '需要 FIN/UID');
    const before = Number(psql(`SELECT coin_balance FROM users WHERE id='${UID}'`));
    // 使余额为负的巨额扣减
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/balance`, {
      headers: { Authorization: `Bearer ${FIN}` },
      data: { delta: -99999999, reason: 'rollback-test' },
    });
    expect([400, 409, 422]).toContain(r.status());
    const after = Number(psql(`SELECT coin_balance FROM users WHERE id='${UID}'`));
    expect(after).toBe(before);
  });
});
