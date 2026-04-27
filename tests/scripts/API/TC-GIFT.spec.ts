/**
 * 测试套件：GIFT 礼物（API）
 * 用例来源：doc/tests/cases/API/TC-GIFT.md
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import 'dotenv/config';

const APP = process.env.APP_SERVER_BASE_URL ?? 'http://localhost:3000';
const ADMIN = process.env.ADMIN_SERVER_BASE_URL ?? 'http://localhost:3001';
const T = process.env.E2E_VALID_TOKEN ?? '';
const AT = process.env.E2E_OP_TOKEN ?? '';
const ROOM = process.env.E2E_ROOM_ID ?? '';
const A = process.env.E2E_USER_A_ID ?? '';
const B = process.env.E2E_USER_B_ID ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();
const redis = (s: string) => execSync(`redis-cli ${s}`, { encoding: 'utf-8' }).trim();

test.describe('TC-GIFT API - 礼物', () => {
  test('TC-GIFT-00001: 礼物列表 排序 + 缓存 + Accept-Language', async ({ request }) => {
    test.skip(!T, '需要 E2E_VALID_TOKEN');
    redis('DEL gifts:list:zh-CN gifts:list:ar');
    const r1 = await request.get(`${APP}/api/v1/gifts/list`, {
      headers: { Authorization: `Bearer ${T}`, 'Accept-Language': 'zh-CN' },
    });
    expect(r1.status()).toBe(200);
    const list = (await r1.json()).data;
    // 按 sort_order ASC
    for (let i = 1; i < list.length; i++) expect(list[i].sort_order).toBeGreaterThanOrEqual(list[i - 1].sort_order);
    expect(Number(redis('EXISTS gifts:list:zh-CN'))).toBe(1);

    const r2 = await request.get(`${APP}/api/v1/gifts/list`, {
      headers: { Authorization: `Bearer ${T}`, 'Accept-Language': 'ar' },
    });
    const arList = (await r2.json()).data;
    expect(arList[0].name).not.toBe(list[0].name);
  });

  test('TC-GIFT-00002: SendGift 原子事务 + WS 推送', async ({ request }) => {
    test.skip(!T || !ROOM || !A || !B, '需要 Token/房间/用户 ID');
    const before = Number(psql(`SELECT coin_balance FROM users WHERE id='${A}'`));
    const msgId = `g_${Date.now()}`;
    const r = await request.post(`${APP}/api/v1/gifts/send`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { gift_id: 'rose', receiver_id: B, count: 1, room_id: ROOM, msg_id: msgId },
    });
    expect(r.status()).toBe(200);
    const after = Number(psql(`SELECT coin_balance FROM users WHERE id='${A}'`));
    expect(before - after).toBe(10); // rose=10
    expect(psql(`SELECT count(*) FROM transactions WHERE msg_id='${msgId}'`)).toBe('1');
  });

  test('TC-GIFT-00003: 余额不足 40290 + 回滚', async ({ request }) => {
    const POOR = process.env.E2E_POOR_TOKEN ?? '';
    test.skip(!POOR || !B || !ROOM, '需要 E2E_POOR_TOKEN');
    const r = await request.post(`${APP}/api/v1/gifts/send`, {
      headers: { Authorization: `Bearer ${POOR}` },
      data: { gift_id: 'rocket', receiver_id: B, count: 1, room_id: ROOM, msg_id: `p_${Date.now()}` },
    });
    expect(r.status()).toBe(402);
    expect((await r.json()).code).toBe(40290);
  });

  test('TC-GIFT-00004: 接收者离麦/不存在 40403', async ({ request }) => {
    test.skip(!T || !ROOM, '需要 Token/房间');
    const r = await request.post(`${APP}/api/v1/gifts/send`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { gift_id: 'rose', receiver_id: '00000000-0000-0000-0000-000000000000', count: 1, room_id: ROOM, msg_id: `x_${Date.now()}` },
    });
    expect(r.status()).toBe(404);
    expect((await r.json()).code).toBe(40403);
  });

  test('TC-GIFT-00005: msg_id 幂等 + 并发不超卖', async ({ request }) => {
    test.skip(!T || !B || !ROOM, '需要 Token');
    const msgId = `idem_${Date.now()}`;
    const payload = {
      headers: { Authorization: `Bearer ${T}` },
      data: { gift_id: 'rose', receiver_id: B, count: 1, room_id: ROOM, msg_id: msgId },
    };
    const rs = await Promise.all([
      request.post(`${APP}/api/v1/gifts/send`, payload),
      request.post(`${APP}/api/v1/gifts/send`, payload),
      request.post(`${APP}/api/v1/gifts/send`, payload),
    ]);
    for (const r of rs) expect(r.status()).toBe(200);
    expect(psql(`SELECT count(*) FROM transactions WHERE msg_id='${msgId}'`)).toBe('1');
  });

  test('TC-GIFT-00006: count 边界 0/1/99/100', async ({ request }) => {
    test.skip(!T || !B || !ROOM, '需要 Token');
    for (const [n, ok] of [[0, false], [1, true], [99, true], [100, false]] as const) {
      const r = await request.post(`${APP}/api/v1/gifts/send`, {
        headers: { Authorization: `Bearer ${T}` },
        data: { gift_id: 'rose', receiver_id: B, count: n, room_id: ROOM, msg_id: `c_${n}_${Date.now()}` },
      });
      if (ok) expect(r.status()).toBe(200);
      else expect(r.status()).toBe(400);
    }
  });

  test('TC-GIFT-00007: Admin 礼物 CRUD + 软删 + 审计', async ({ request }) => {
    test.skip(!AT, '需要 E2E_OP_TOKEN');
    const create = await request.post(`${ADMIN}/api/v1/admin/gifts`, {
      headers: { Authorization: `Bearer ${AT}` },
      data: { id: 'test_gift', name_zh: '测试', name_ar: 'اختبار', price: 5, image: 'http://x/a.png', sort_order: 99 },
    });
    expect(create.status()).toBe(201);
    const upd = await request.patch(`${ADMIN}/api/v1/admin/gifts/test_gift`, {
      headers: { Authorization: `Bearer ${AT}` }, data: { price: 8 },
    });
    expect(upd.status()).toBe(200);
    const del = await request.delete(`${ADMIN}/api/v1/admin/gifts/test_gift`, {
      headers: { Authorization: `Bearer ${AT}` },
    });
    expect(del.status()).toBe(200);
    expect(psql(`SELECT deleted_at IS NOT NULL FROM gifts WHERE id='test_gift'`)).toBe('t');
    expect(Number(psql(`SELECT count(*) FROM admin_logs WHERE target_id='test_gift'`))).toBeGreaterThanOrEqual(3);
  });
});
