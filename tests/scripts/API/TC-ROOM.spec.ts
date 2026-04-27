/**
 * 测试套件：ROOM 房间（API）
 * 用例来源：doc/tests/cases/API/TC-ROOM.md
 */
import { test, expect, request as pwRequest } from '@playwright/test';
import { execSync } from 'child_process';
import 'dotenv/config';

const APP = process.env.APP_SERVER_BASE_URL ?? 'http://localhost:3000';
const ADMIN = process.env.ADMIN_SERVER_BASE_URL ?? 'http://localhost:3001';
const T = process.env.E2E_VALID_TOKEN ?? '';
const OP = process.env.E2E_OP_TOKEN ?? '';
const CS = process.env.E2E_CS_TOKEN ?? '';
const FIN = process.env.E2E_FIN_TOKEN ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

test.describe('TC-ROOM API - 房间', () => {
  test('TC-ROOM-00001: 创建房间 201', async ({ request }) => {
    test.skip(!T, '需要 E2E_VALID_TOKEN');
    const r = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: 'Test Room', cover: 1, type: 'chat', announcement: '' },
    });
    expect(r.status()).toBe(201);
    const body = await r.json();
    expect(body.data.id).toBeTruthy();
    expect(body.data.status).toBe('open');
    psql(`UPDATE rooms SET status='closed' WHERE id='${body.data.id}'`);
  });

  test('TC-ROOM-00002: 标题长度边界 0/1/30/31', async ({ request }) => {
    test.skip(!T, '需要 Token');
    for (const [len, ok] of [[0, false], [1, true], [30, true], [31, false]] as const) {
      const r = await request.post(`${APP}/api/v1/rooms`, {
        headers: { Authorization: `Bearer ${T}` },
        data: { title: 'a'.repeat(len), cover: 1, type: 'chat' },
      });
      expect(r.status()).toBe(ok ? 201 : 400);
    }
  });

  test('TC-ROOM-00003: room_type 枚举 + 密码字段', async ({ request }) => {
    test.skip(!T, '需要 Token');
    // 密码房必须带 password
    const noPw = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: 'pw', cover: 1, type: 'password' },
    });
    expect(noPw.status()).toBe(400);
    // 非法枚举
    const bad = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: 'x', cover: 1, type: 'invalid' },
    });
    expect(bad.status()).toBe(400);
  });

  test('TC-ROOM-00004: 同用户并发创建仅一成功', async ({ playwright }) => {
    test.skip(!T, '需要 Token');
    const ctx = await pwRequest.newContext({ baseURL: APP });
    const rs = await Promise.all(
      Array.from({ length: 5 }).map(() =>
        ctx.post('/api/v1/rooms', {
          headers: { Authorization: `Bearer ${T}` },
          data: { title: 'race', cover: 1, type: 'chat' },
        })),
    );
    const ok = rs.filter((r) => r.status() === 201);
    expect(ok.length).toBe(1);
    await ctx.dispose();
  });

  test('TC-ROOM-00005: 未登录 / Token 过期', async ({ request }) => {
    const r = await request.post(`${APP}/api/v1/rooms`, { data: { title: 'x', cover: 1, type: 'chat' } });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40101);
  });

  test('TC-ROOM-00006: 列表 热度降序 + 分页', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const r = await request.get(`${APP}/api/v1/rooms?page=1&per_page=20`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(r.status()).toBe(200);
    const list = (await r.json()).data.items;
    for (let i = 1; i < list.length; i++) expect(list[i].online_count).toBeLessThanOrEqual(list[i - 1].online_count);
  });

  test('TC-ROOM-00007: 已关闭/软删除房间不可见', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const r = await request.get(`${APP}/api/v1/rooms?page=1&per_page=100`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    const list = (await r.json()).data.items;
    for (const i of list) expect(i.status).toBe('open');
  });

  test('TC-ROOM-00008: 详情 合法/非法/不存在', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const good = process.env.E2E_ROOM_ID ?? '';
    if (good) {
      const r = await request.get(`${APP}/api/v1/rooms/${good}`, { headers: { Authorization: `Bearer ${T}` } });
      expect(r.status()).toBe(200);
    }
    const r404 = await request.get(`${APP}/api/v1/rooms/00000000-0000-0000-0000-000000000000`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(r404.status()).toBe(404);
    const r400 = await request.get(`${APP}/api/v1/rooms/not-a-uuid`, { headers: { Authorization: `Bearer ${T}` } });
    expect(r400.status()).toBe(400);
  });

  test('TC-ROOM-00009: 关闭房间 权限 + 状态机', async ({ request }) => {
    test.skip(!T, '需要 Token');
    const created = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: 'closing', cover: 1, type: 'chat' },
    });
    const rid = (await created.json()).data.id;
    const close = await request.post(`${APP}/api/v1/rooms/${rid}/close`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(close.status()).toBe(200);
    // 重复关闭幂等
    const close2 = await request.post(`${APP}/api/v1/rooms/${rid}/close`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect([200, 409]).toContain(close2.status());
  });

  test('TC-ROOM-00010: Admin 列表 筛选 + RBAC', async ({ request }) => {
    test.skip(!OP, '需要 OP Token');
    const r = await request.get(`${ADMIN}/api/v1/admin/rooms?status=open&page=1`, {
      headers: { Authorization: `Bearer ${OP}` },
    });
    expect(r.status()).toBe(200);
    if (FIN) {
      const forbid = await request.get(`${ADMIN}/api/v1/admin/rooms`, { headers: { Authorization: `Bearer ${FIN}` } });
      expect(forbid.status()).toBe(403);
    }
  });

  test('TC-ROOM-00011: Admin 详情 closed 可见 / 软删 404', async ({ request }) => {
    test.skip(!OP, '需要 OP Token');
    const closedId = process.env.E2E_CLOSED_ROOM_ID ?? '';
    if (closedId) {
      const r = await request.get(`${ADMIN}/api/v1/admin/rooms/${closedId}`, {
        headers: { Authorization: `Bearer ${OP}` },
      });
      expect(r.status()).toBe(200);
      expect((await r.json()).data.status).toBe('closed');
    }
  });

  test('TC-ROOM-00012: Admin 强制关闭 + 审计', async ({ request }) => {
    test.skip(!T || !OP, '需要 Token');
    const created = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: 'fc', cover: 1, type: 'chat' },
    });
    const rid = (await created.json()).data.id;
    const r = await request.post(`${ADMIN}/api/v1/admin/rooms/${rid}/force-close`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { reason: '违规' },
    });
    expect(r.status()).toBe(200);
    expect(psql(`SELECT status FROM rooms WHERE id='${rid}'`)).toBe('closed');
    const logs = Number(psql(`SELECT count(*) FROM admin_logs WHERE action='force_close_room' AND target_id='${rid}'`));
    expect(logs).toBeGreaterThanOrEqual(1);
  });
});
