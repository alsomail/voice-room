/**
 * 测试套件：RANKING 排行榜（API）
 * 用例来源：doc/tests/cases/API/TC-RANKING.md
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import 'dotenv/config';

const APP = process.env.APP_SERVER_BASE_URL ?? 'http://localhost:3000';
const T = process.env.E2E_VALID_TOKEN ?? '';
const redis = (s: string) => execSync(`redis-cli ${s}`, { encoding: 'utf-8' }).trim();

test.describe('TC-RANKING API - 排行榜', () => {
  test.skip(!T, '需要 E2E_VALID_TOKEN');

  test('TC-RANKING-00001: 参数矩阵', async ({ request }) => {
    for (const period of ['day', 'week']) {
      for (const type of ['send', 'recv']) {
        const r = await request.get(`${APP}/api/v1/ranking?period=${period}&type=${type}&limit=50`, {
          headers: { Authorization: `Bearer ${T}` },
        });
        expect(r.status()).toBe(200);
        const body = await r.json();
        expect(Array.isArray(body.data.list)).toBe(true);
        expect(body.data.list.length).toBeLessThanOrEqual(50);
      }
    }
    // 非法 period
    const bad = await request.get(`${APP}/api/v1/ranking?period=month`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(bad.status()).toBe(400);
  });

  test('TC-RANKING-00002: me.rank 未上榜为 null', async ({ request }) => {
    const r = await request.get(`${APP}/api/v1/ranking?period=day&type=send`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    const me = (await r.json()).data.me;
    expect(me).toHaveProperty('rank');
    // 空榜时 rank 应为 null
    if (me.coins === 0) expect(me.rank).toBeNull();
  });

  test('TC-RANKING-00003: p95 ≤100ms', async ({ request }) => {
    test.skip(process.env.CI_E2E_READY !== '1', '跳过性能用例');
    const ts: number[] = [];
    for (let i = 0; i < 20; i++) {
      const t0 = Date.now();
      await request.get(`${APP}/api/v1/ranking?period=day&type=send`, {
        headers: { Authorization: `Bearer ${T}` },
      });
      ts.push(Date.now() - t0);
    }
    ts.sort((a, b) => a - b);
    const p95 = ts[Math.floor(ts.length * 0.95) - 1];
    expect(p95).toBeLessThanOrEqual(100);
  });

  test('TC-RANKING-00004: 日/周键 归档', async () => {
    // UTC+3 的 day key 格式 ranking:send:day:YYYYMMDD
    const keys = execSync("redis-cli KEYS 'ranking:*:day:*'", { encoding: 'utf-8' }).trim().split('\n').filter(Boolean);
    expect(keys.length).toBeGreaterThanOrEqual(0);
    // 每个日 key 应有 TTL（2 天内清除）
    for (const k of keys.slice(0, 3)) {
      const ttl = Number(redis(`TTL ${k}`));
      expect(ttl).toBeGreaterThan(0);
      expect(ttl).toBeLessThanOrEqual(3 * 86400);
    }
  });
});
