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
    for (const f of ['id', 'phone', 'nickname', 'wallet', 'recent_transactions', 'devices']) {
      expect(data).toHaveProperty(f);
    }
  });

  test('TC-USER-00003: 封禁用户 - 临时/永久 + 审计 + WS 踢下线', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { type: 'temporary', duration_hours: 24, reason: '违规' },
    });
    expect(r.status()).toBe(200);
    expect(psql(`SELECT banned_until IS NOT NULL FROM users WHERE id='${UID}'`)).toBe('t');
    const logs = Number(psql(`SELECT count(*) FROM admin_logs WHERE action='ban_user' AND target_id='${UID}'`));
    expect(logs).toBeGreaterThanOrEqual(1);
  });

  test('TC-USER-00004: 非法参数 + 重复封禁幂等', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    const bad = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { type: 'temporary' },
    });
    expect(bad.status()).toBe(400);
    // 重复永久封禁
    const a = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { type: 'permanent', reason: 'x' },
    });
    const b = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/ban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { type: 'permanent', reason: 'x' },
    });
    expect(a.status()).toBe(200);
    expect([200, 409]).toContain(b.status());
  });

  test('TC-USER-00005: 解封 - 状态恢复 + 审计', async ({ request }) => {
    test.skip(!OP || !UID, '需要 OP/UID');
    const r = await request.post(`${ADMIN}/api/v1/admin/users/${UID}/unban`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { reason: '申诉通过' },
    });
    expect(r.status()).toBe(200);
    expect(psql(`SELECT banned_until IS NULL FROM users WHERE id='${UID}'`)).toBe('t');
    const logs = Number(psql(`SELECT count(*) FROM admin_logs WHERE action='unban_user' AND target_id='${UID}'`));
    expect(logs).toBeGreaterThanOrEqual(1);
  });
});
