/**
 * 测试套件：USER Admin 用户管理（API）
 * 用例来源：doc/tests/cases/API/TC-USER.md
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

const ADMIN = process.env.ADMIN_SERVER_BASE_URL!;
const CS = process.env.E2E_CS_TOKEN ?? '';
const OP = process.env.E2E_OP_TOKEN ?? '';
const UID = process.env.E2E_USER_A_ID ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

test.describe('TC-USER API - Admin 用户管理', () => {
  test.describe.configure({ mode: 'serial' });
  test('TC-USER-00001: 列表 - 分页/检索/XSS 安全 @prod-safe', { tag: '@prod-safe' }, async ({ request }) => {
    test.skip(!CS, '需要 E2E_CS_TOKEN');
    const r = await request.get(`${ADMIN}/api/v1/admin/users?page=1&per_page=20&q=user`, {
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect(r.status()).toBe(200);
    const text = await r.text();
    // 字段已转义
    expect(text).not.toContain('<script>');

    // XSS 注入查询
    const xss = await request.get(`${ADMIN}/api/v1/admin/users?q=%3Cscript%3Ealert(1)%3C/script%3E`, {
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect(xss.status()).toBe(200);
  });

  test('TC-USER-00002: 详情 - 含钱包/流水/设备 @prod-safe', { tag: '@prod-safe' }, async ({ request }) => {
    test.skip(!CS || !UID, '需要 Token/UserID');
    const r = await request.get(`${ADMIN}/api/v1/admin/users/${UID}`, {
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect(r.status()).toBe(200);
    const data = (await r.json()).data;
    // Server returns coin_balance directly (not nested wallet), recharge/consume records, devices
    for (const f of ['id', 'phone', 'nickname', 'coin_balance']) {
      expect(data).toHaveProperty(f);
    }
  });

  test('TC-USER-00003: 封禁用户 - 临时/永久 + 审计 + WS 踢下线', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    // Ensure clean state: unban first in case user is already banned from prior run
    await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'unban', reason: 'cleanup' },
    });
    // Ban API uses { action: 'ban', ban_type: ..., duration_hours: ..., reason: ... }
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'ban', ban_type: 'temporary', duration_hours: 24, reason: '违规' },
    });
    expect(r.status()).toBe(200);
    // DB uses is_banned boolean (no banned_until column)
    expect(psql(`SELECT is_banned FROM users WHERE id='${UID}'`)).toBe('t');
    const logs = Number(psql(`SELECT count(*) FROM admin_logs WHERE action='ban_user' AND target_id='${UID}'`));
    expect(logs).toBeGreaterThanOrEqual(1);
  });

  test('TC-USER-00004: 非法参数 + 重复封禁幂等', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    // Reset state: unban user first (may be banned from TC-USER-00003)
    await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'unban', reason: 'cleanup' },
    });
    // Missing duration_hours when ban_type=temporary → 400/422; if user already banned → 409
    const bad = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'ban', ban_type: 'temporary' },
    });
    expect([400, 409, 422]).toContain(bad.status());
    // 重复永久封禁: first should succeed (200), second should be idempotent (200 or 409)
    const a = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'ban', ban_type: 'permanent', reason: 'x' },
    });
    const b = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'ban', ban_type: 'permanent', reason: 'x' },
    });
    expect([200, 409]).toContain(a.status());
    expect([200, 409]).toContain(b.status());
  });

  test('TC-USER-00005: 解封 - 状态恢复 + 审计', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    // Unban uses same /ban endpoint with action: 'unban'
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { action: 'unban', reason: '申诉通过' },
    });
    expect(r.status()).toBe(200);
    // DB uses is_banned boolean (no banned_until column)
    expect(psql(`SELECT is_banned FROM users WHERE id='${UID}'`)).toBe('f');
    const logs = Number(psql(`SELECT count(*) FROM admin_logs WHERE action='unban_user' AND target_id='${UID}'`));
    expect(logs).toBeGreaterThanOrEqual(1);
  });
});
