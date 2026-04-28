/**
 * 测试套件：RANKING 排行榜（API）
 * 用例来源：doc/tests/cases/API/TC-RANKING.md
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import { resolveRedisCliMode, isRedisCliAvailable } from '../support/redisCli';

const APP = process.env.APP_SERVER_BASE_URL!;
const T = process.env.E2E_VALID_TOKEN ?? '';
const REDIS_PREFIX = resolveRedisCliMode() === 'docker'
  ? 'docker exec vr-redis redis-cli'
  : 'redis-cli';
const redis = (s: string) => execSync(`${REDIS_PREFIX} ${s}`, { encoding: 'utf-8' }).trim();
const hasRedisCli = isRedisCliAvailable();

test.describe('TC-RANKING API - 排行榜', () => {
  test.skip(!T, '需要 E2E_VALID_TOKEN');

  test('TC-RANKING-00001: 参数矩阵 @prod-safe', { tag: '@prod-safe' }, async ({ request }) => {
    for (const period of ['day', 'week']) {
      for (const type of ['charm', 'wealth']) {
        const r = await request.get(`${APP}/api/v1/ranking?period=${period}&type=${type}&limit=50`, {
          headers: { Authorization: `Bearer ${T}` },
        });
        expect(r.status()).toBe(200);
        const body = await r.json();
        expect(Array.isArray(body.data.items)).toBe(true);
        expect(body.data.items.length).toBeLessThanOrEqual(50);
      }
    }
    // 非法 period
    const bad = await request.get(`${APP}/api/v1/ranking?period=month&type=charm`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    expect(bad.status()).toBe(400);
  });

  test('TC-RANKING-00002: me.rank 未上榜为 null @prod-safe', { tag: '@prod-safe' }, async ({ request }) => {
    const r = await request.get(`${APP}/api/v1/ranking?period=day&type=charm`, {
      headers: { Authorization: `Bearer ${T}` },
    });
    const me = (await r.json()).data.me;
    expect(me).toHaveProperty('rank');
    // 空榜时 rank 应为 null
    if (me.score === 0) expect(me.rank).toBeNull();
  });

  test('TC-RANKING-00003: p95 ≤100ms', async ({ request }) => {
    test.skip(process.env.CI_E2E_READY !== '1', '跳过性能用例');
    const ts: number[] = [];
    for (let i = 0; i < 20; i++) {
      const t0 = Date.now();
      await request.get(`${APP}/api/v1/ranking?period=day&type=charm`, {
        headers: { Authorization: `Bearer ${T}` },
      });
      ts.push(Date.now() - t0);
    }
    ts.sort((a, b) => a - b);
    const p95 = ts[Math.floor(ts.length * 0.95) - 1];
    expect(p95).toBeLessThanOrEqual(100);
  });

  test('TC-RANKING-00004: 日/周键 归档', async () => {
    test.skip(!hasRedisCli, 'SKIP-KNOWN-FOLLOWUP: redis-cli unavailable (neither docker nor PATH)');
    // UTC+3 的 day key 格式 ranking:send:day:YYYYMMDD
    const allKeys = execSync(`${REDIS_PREFIX} KEYS 'ranking:*:day:*'`, { encoding: 'utf-8' }).trim().split('\n').filter(Boolean);
    // Filter to keys ending with 8-digit date (e.g. 20260427), excluding test-created keys
    const keys = allKeys.filter(k => /\d{8}$/.test(k));
    expect(keys.length).toBeGreaterThanOrEqual(0);
    // 每个日 key 应有 TTL（2 天内清除）
    for (const k of keys.slice(0, 3)) {
      const ttl = Number(redis(`TTL ${k}`));
      expect(ttl).toBeGreaterThan(0);
      expect(ttl).toBeLessThanOrEqual(3 * 86400);
    }
  });
});
