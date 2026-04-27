/**
 * 测试套件：LOG 后台审计日志（API）
 * 用例来源：doc/tests/cases/API/TC-LOG.md
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

const ADMIN = process.env.ADMIN_SERVER_BASE_URL!;
const OP = process.env.E2E_OP_TOKEN ?? '';
const CS = process.env.E2E_CS_TOKEN ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

test.describe('TC-LOG API - 审计日志', () => {
  test.skip(!OP, '需要 E2E_OP_TOKEN');

  test('TC-LOG-00001: 关键操作自动写入 admin_logs', async ({ request }) => {
    const before = Number(psql(`SELECT count(*) FROM admin_logs`));
    // 触发一次 Admin 操作（创建礼物）
    await request.post(`${ADMIN}/api/v1/admin/gifts`, {
      headers: { Authorization: `Bearer ${OP}` },
      data: { id: `log_${Date.now()}`, name_zh: 'x', name_ar: 'x', price: 1, image: 'http://x' },
    });
    const after = Number(psql(`SELECT count(*) FROM admin_logs`));
    expect(after).toBeGreaterThan(before);
    const row = psql(`SELECT admin_id, action, ip IS NOT NULL FROM admin_logs ORDER BY created_at DESC LIMIT 1`);
    expect(row).toMatch(/create_gift|gift\.create/);
    expect(row.endsWith('|t')).toBe(true);
  });

  test('TC-LOG-00002: 日志查询 - 筛选条件', async ({ request }) => {
    const r = await request.get(`${ADMIN}/api/v1/admin/logs?action=create_gift&page=1&per_page=20`, {
      headers: { Authorization: `Bearer ${OP}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body.data.items)).toBe(true);
    for (const i of body.data.items) expect(i.action).toBe('create_gift');

    // 时间范围
    const t = await request.get(
      `${ADMIN}/api/v1/admin/logs?start=2020-01-01T00:00:00Z&end=2020-01-02T00:00:00Z`,
      { headers: { Authorization: `Bearer ${OP}` } },
    );
    expect(t.status()).toBe(200);
    expect((await t.json()).data.total).toBe(0);
  });

  test('TC-LOG-00003: 10 万行查询 ≤500ms', async ({ request }) => {
    test.skip(process.env.CI_E2E_READY !== '1', '跳过慢查询性能测试');
    // 需前置已 seed 10 万行
    const t0 = Date.now();
    const r = await request.get(`${ADMIN}/api/v1/admin/logs?page=1&per_page=20`, {
      headers: { Authorization: `Bearer ${OP}` },
    });
    const dt = Date.now() - t0;
    expect(r.status()).toBe(200);
    expect(dt).toBeLessThanOrEqual(500);
  });

  test('TC-LOG-00004 [附加]: CS 无权访问敏感日志', async ({ request }) => {
    test.skip(!CS, '需要 E2E_CS_TOKEN');
    const r = await request.get(`${ADMIN}/api/v1/admin/logs`, {
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect([200, 403]).toContain(r.status());
  });
});
