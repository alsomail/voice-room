/**
 * 测试套件：AUTH 用户认证（API）
 * 用例来源：doc/tests/cases/API/TC-AUTH.md
 * 说明：
 *   - API 层用例直接通过 Playwright 的 `request` fixture 进行 HTTP 断言，不启动浏览器。
 *   - Redis/DB 操作通过 execSync 调用本机 redis-cli / psql（需已在环境变量中配置）。
 *   - 所有用例假设 App Server / Admin Server 由 envLoader 注入（base URL 必填）。
 */
import { test, expect, request as playwrightRequest } from '@playwright/test';
import { execSync } from 'child_process';

const APP_BASE = process.env.APP_SERVER_BASE_URL!;
const ADMIN_BASE = process.env.ADMIN_SERVER_BASE_URL!;

const redis = (cmd: string): string =>
  execSync(`redis-cli ${cmd}`, { encoding: 'utf-8' }).trim();

const psql = (sql: string): string =>
  execSync(
    `psql "${process.env.DATABASE_URL!}" -tA -c "${sql.replace(/"/g, '\\"')}"`,
    { encoding: 'utf-8' },
  ).trim();

test.describe('TC-AUTH API - 用户认证', () => {
  test('TC-AUTH-00001: 发送验证码 - 合法沙特手机号首次成功', async ({ request }) => {
    const phone = '+966512345678';
    redis(`DEL sms:code:${phone} sms:cooldown:${phone} sms:daily:${phone}`);

    const res = await request.post(`${APP_BASE}/api/v1/auth/verification-codes`, {
      data: { phone },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(0);
    expect(body.data.expires_in).toBe(300);
    expect(body.data.cooldown).toBe(60);

    const code = redis(`GET sms:code:${phone}`);
    expect(code).toMatch(/^\d{6}$/);
    const codeTtl = Number(redis(`TTL sms:code:${phone}`));
    expect(codeTtl).toBeGreaterThanOrEqual(295);
    expect(codeTtl).toBeLessThanOrEqual(300);
    const coolTtl = Number(redis(`TTL sms:cooldown:${phone}`));
    expect(coolTtl).toBeGreaterThanOrEqual(55);
    expect(coolTtl).toBeLessThanOrEqual(60);

    redis(`DEL sms:code:${phone} sms:cooldown:${phone}`);
  });

  test('TC-AUTH-00002: 验证码 60s 冷却 42901', async ({ request }) => {
    const phone = '+966512345678';
    redis(`SET sms:cooldown:${phone} 1 EX 30`);
    redis(`SET sms:code:${phone} 111111 EX 300`);

    const res = await request.post(`${APP_BASE}/api/v1/auth/verification-codes`, {
      data: { phone },
    });
    expect(res.status()).toBe(429);
    const body = await res.json();
    expect(body.code).toBe(42901);
    expect(String(body.message)).toMatch(/too frequently|frequent/i);

    expect(redis(`GET sms:code:${phone}`)).toBe('111111');
    redis(`DEL sms:code:${phone} sms:cooldown:${phone}`);
  });

  test('TC-AUTH-00003: 每日限额边界值 Max=10 / Max+1=11', async ({ request }) => {
    const phone = '+966512345678';
    redis(`DEL sms:cooldown:${phone}`);
    redis(`SET sms:daily:${phone} 9 EX 86400`);

    // 第 10 次 Max
    let res = await request.post(`${APP_BASE}/api/v1/auth/verification-codes`, {
      data: { phone },
    });
    expect(res.status()).toBe(200);
    expect(redis(`GET sms:daily:${phone}`)).toBe('10');

    // 清除 cooldown 后第 11 次 Max+1
    redis(`DEL sms:cooldown:${phone}`);
    res = await request.post(`${APP_BASE}/api/v1/auth/verification-codes`, {
      data: { phone },
    });
    expect(res.status()).toBe(429);
    expect((await res.json()).code).toBe(42902);
    expect(redis(`GET sms:daily:${phone}`)).toBe('10');

    redis(`DEL sms:daily:${phone} sms:cooldown:${phone} sms:code:${phone}`);
  });

  test('TC-AUTH-00004: 手机号格式等价类覆盖', async ({ request }) => {
    const badCases: Array<{ body: Record<string, unknown>; allowCodes: number[] }> = [
      { body: { phone: '12345678' }, allowCodes: [40001] },
      { body: { phone: '+966abc12345' }, allowCodes: [40001] },
      { body: { phone: '' }, allowCodes: [40001, 40002] },
      { body: {}, allowCodes: [40002] },
      { body: { phone: '+9665123456789012345' }, allowCodes: [40001] },
      { body: { phone: "' OR '1'='1" }, allowCodes: [40001] },
    ];
    for (const c of badCases) {
      const res = await request.post(`${APP_BASE}/api/v1/auth/verification-codes`, {
        data: c.body,
      });
      expect(res.status()).toBe(400);
      const json = await res.json();
      expect(c.allowCodes).toContain(json.code);
    }
  });

  test('TC-AUTH-00005: 新用户自动注册 & JWT 签发', async ({ request }) => {
    const phone = '+966500000001';
    psql(`DELETE FROM users WHERE phone='${phone}'`);
    redis(`SET sms:code:${phone} 123456 EX 300`);

    const res = await request.post(`${APP_BASE}/api/v1/auth/login`, {
      data: { phone, code: '123456' },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(typeof body.data.token).toBe('string');
    expect(body.data.token.length).toBeGreaterThan(20);
    expect(body.data.user.is_new).toBe(true);
    expect(body.data.user.nickname).toMatch(/^User_[a-z0-9]{4}$/);
    expect(body.data.user.coin_balance).toBe(0);

    const row = psql(
      `SELECT id, coin_balance, deleted_at FROM users WHERE phone='${phone}'`,
    );
    expect(row).not.toBe('');
    expect(row.split('|')[1]).toBe('0');

    expect(redis(`EXISTS sms:code:${phone}`)).toBe('0');

    const me = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: `Bearer ${body.data.token}` },
    });
    expect(me.status()).toBe(200);
    expect((await me.json()).data.id).toBe(body.data.user.id);

    psql(`DELETE FROM users WHERE phone='${phone}'`);
  });

  test('TC-AUTH-00006: 验证码错误 5 次锁定 40105', async ({ request }) => {
    const phone = '+966500000002';
    redis(`SET sms:code:${phone} 111111 EX 300`);
    redis(`DEL sms:attempts:${phone}`);

    for (let i = 1; i <= 5; i++) {
      const res = await request.post(`${APP_BASE}/api/v1/auth/login`, {
        data: { phone, code: '222222' },
      });
      expect(res.status()).toBe(401);
      expect((await res.json()).code).toBe(40103);
    }
    expect(redis(`GET sms:attempts:${phone}`)).toBe('5');

    const res6 = await request.post(`${APP_BASE}/api/v1/auth/login`, {
      data: { phone, code: '222222' },
    });
    expect(res6.status()).toBe(401);
    expect((await res6.json()).code).toBe(40105);

    // 即便使用正确码也不放行
    const res7 = await request.post(`${APP_BASE}/api/v1/auth/login`, {
      data: { phone, code: '111111' },
    });
    expect(res7.status()).toBe(401);
    expect((await res7.json()).code).toBe(40105);

    expect(psql(`SELECT count(*) FROM users WHERE phone='${phone}'`)).toBe('0');
    redis(`DEL sms:code:${phone} sms:attempts:${phone}`);
  });

  test('TC-AUTH-00007: 验证码已过期 40104', async ({ request }) => {
    const phone = '+966500000003';
    redis(`DEL sms:code:${phone}`);
    const res = await request.post(`${APP_BASE}/api/v1/auth/login`, {
      data: { phone, code: '123456' },
    });
    expect(res.status()).toBe(401);
    expect((await res.json()).code).toBe(40104);
  });

  test('TC-AUTH-00008: JWT 中间件 - 缺失/非法/过期/iss', async ({ request }) => {
    const VALID = process.env.E2E_VALID_TOKEN ?? '';
    const EXPIRED = process.env.E2E_EXPIRED_TOKEN ?? '';
    const ADMIN = process.env.E2E_ADMIN_TOKEN ?? '';
    test.skip(!VALID || !EXPIRED || !ADMIN, '需要 E2E_VALID_TOKEN/E2E_EXPIRED_TOKEN/E2E_ADMIN_TOKEN');

    let r = await request.get(`${APP_BASE}/api/v1/users/me`);
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40101);

    r = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: 'Bearer abc.def.ghi' },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40101);

    r = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: `Bearer ${EXPIRED}` },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40102);

    r = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: `Bearer ${ADMIN}` },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40101);

    r = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: `Bearer ${VALID}` },
    });
    expect(r.status()).toBe(200);
  });

  test('TC-AUTH-00009: /users/me 响应不含敏感字段', async ({ request }) => {
    const TOKEN = process.env.E2E_VALID_TOKEN ?? '';
    test.skip(!TOKEN, '需要 E2E_VALID_TOKEN');

    const r = await request.get(`${APP_BASE}/api/v1/users/me`, {
      headers: { Authorization: `Bearer ${TOKEN}` },
    });
    expect(r.status()).toBe(200);
    const text = await r.text();
    for (const field of ['password', 'password_hash', 'deleted_at', 'updated_at']) {
      expect(text).not.toContain(field);
    }
    const data = (await r.json()).data;
    for (const field of ['id', 'phone', 'nickname', 'avatar', 'coin_balance', 'vip_level', 'created_at']) {
      expect(data).toHaveProperty(field);
    }
  });

  test('TC-AUTH-00010: 登录幂等 5 并发仅注册 1 账号', async ({ playwright }) => {
    const phone = '+966500000010';
    psql(`DELETE FROM users WHERE phone='${phone}'`);
    redis(`SET sms:code:${phone} 888888 EX 300`);

    const ctx = await playwrightRequest.newContext({ baseURL: APP_BASE });
    const results = await Promise.all(
      Array.from({ length: 5 }).map(() =>
        ctx.post('/api/v1/auth/login', { data: { phone, code: '888888' } }),
      ),
    );
    const bodies = await Promise.all(results.map(r => r.json()));
    for (const r of results) expect(r.status()).toBe(200);
    const userIds = new Set(bodies.map(b => b.data.user.id));
    expect(userIds.size).toBe(1);

    expect(psql(`SELECT count(*) FROM users WHERE phone='${phone}'`)).toBe('1');

    psql(`DELETE FROM users WHERE phone='${phone}'`);
    redis(`DEL sms:code:${phone}`);
    await ctx.dispose();
  });

  test('TC-AUTH-00011: Admin 登录 - 正确凭证签发 7 天 JWT', async ({ request }) => {
    const res = await request.post(`${ADMIN_BASE}/api/v1/admin/login`, {
      data: { username: 'admin_op', password: 'Pass@123' },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(typeof body.data.token).toBe('string');
    expect(body.data.admin.role).toBe('operator');
    expect(body.data.expires_in).toBe(604800);

    // 解码 JWT payload（base64url）
    const payload = JSON.parse(
      Buffer.from(body.data.token.split('.')[1], 'base64url').toString('utf-8'),
    );
    expect(payload.iss).toBe('voiceroom-admin');
    expect(payload.role).toBe('operator');
    expect(payload.exp - payload.iat).toBe(604800);

    const logs = psql(
      `SELECT action FROM admin_logs WHERE admin_id=(SELECT id FROM admins WHERE username='admin_op') ORDER BY created_at DESC LIMIT 1`,
    );
    expect(logs).toBe('login');
  });

  test('TC-AUTH-00012: Admin 登录 - 错误凭证/禁用/注入', async ({ request }) => {
    let r = await request.post(`${ADMIN_BASE}/api/v1/admin/login`, {
      data: { username: 'admin_op', password: 'wrong' },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40106);

    r = await request.post(`${ADMIN_BASE}/api/v1/admin/login`, {
      data: { username: 'not_exist', password: 'x' },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40106);

    r = await request.post(`${ADMIN_BASE}/api/v1/admin/login`, {
      data: { username: 'admin_disabled', password: 'Pass@123' },
    });
    expect(r.status()).toBe(403);
    expect((await r.json()).code).toBe(40302);

    r = await request.post(`${ADMIN_BASE}/api/v1/admin/login`, {
      data: { username: "' OR '1'='1", password: 'x' },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40106);
  });

  test('TC-AUTH-00013: Admin JWT 中间件 + RBAC 权限矩阵', async ({ request }) => {
    const CS = process.env.E2E_CS_TOKEN ?? '';
    const OP = process.env.E2E_OP_TOKEN ?? '';
    const FIN = process.env.E2E_FIN_TOKEN ?? '';
    const USER = process.env.E2E_VALID_TOKEN ?? '';
    test.skip(!CS || !OP || !FIN || !USER, '需要 CS/OP/FIN/USER 四类 token');

    let r = await request.get(`${ADMIN_BASE}/api/v1/admin/users`, {
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect(r.status()).toBe(200);

    r = await request.post(`${ADMIN_BASE}/api/v1/admin/users/00000000-0000-0000-0000-000000000000/ban`, {
      data: { type: 'permanent', reason: 'x' },
      headers: { Authorization: `Bearer ${CS}` },
    });
    expect(r.status()).toBe(403);
    expect((await r.json()).code).toBe(40301);

    r = await request.get(`${ADMIN_BASE}/api/v1/admin/rooms`, {
      headers: { Authorization: `Bearer ${FIN}` },
    });
    expect(r.status()).toBe(403);
    expect((await r.json()).code).toBe(40301);

    r = await request.get(`${ADMIN_BASE}/api/v1/admin/stats/overview`, {
      headers: { Authorization: `Bearer ${FIN}` },
    });
    expect(r.status()).toBe(200);

    r = await request.get(`${ADMIN_BASE}/api/v1/admin/me`, {
      headers: { Authorization: `Bearer ${USER}` },
    });
    expect(r.status()).toBe(401);
    expect((await r.json()).code).toBe(40101);

    r = await request.get(`${ADMIN_BASE}/api/v1/admin/me`);
    expect(r.status()).toBe(401);
  });
});
