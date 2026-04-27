/**
 * T-20020 U3.* / U5.2 启动期校验单测
 *
 * 关键技巧：
 *   - vitest 的 `vi.stubEnv` 自 v0.26 起同时影响 `import.meta.env`；
 *   - 模块顶层 `webEnv = readWebEnv()` 在首次 import 时执行；
 *     需借助 `vi.resetModules()` 让每个用例都重新 import 一次 `./env`。
 *   - `src/test/setup.ts` 已为四字段注入默认 stub，本文件需在 beforeEach
 *     主动 `vi.unstubAllEnvs()` 清空，再按场景精确 stub。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const ALL_KEYS = [
  'VITE_API_BASE_URL',
  'VITE_WS_URL',
  'VITE_ADMIN_API_BASE_URL',
  'VITE_ANALYTICS_ENDPOINT',
] as const;

function stubAll(values: Partial<Record<(typeof ALL_KEYS)[number], string>>) {
  const defaults: Record<(typeof ALL_KEYS)[number], string> = {
    VITE_API_BASE_URL: 'http://127.0.0.1:3000/api',
    VITE_WS_URL: 'ws://127.0.0.1:3000/ws',
    VITE_ADMIN_API_BASE_URL: 'http://127.0.0.1:3001/api/v1/admin',
    VITE_ANALYTICS_ENDPOINT: 'https://analytics-test.example.com/collect',
  };
  for (const k of ALL_KEYS) {
    const v = values[k] ?? defaults[k];
    vi.stubEnv(k, v);
  }
}

beforeEach(() => {
  vi.resetModules();
  vi.unstubAllEnvs();
});

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('webEnv 启动期校验', () => {
  it('U3.1 缺 VITE_ADMIN_API_BASE_URL 时 import 抛 [CONFIG ERROR]', async () => {
    stubAll({ VITE_ADMIN_API_BASE_URL: '' });
    await expect(import('./env')).rejects.toThrow(
      '[CONFIG ERROR] VITE_ADMIN_API_BASE_URL must be set',
    );
  });

  it('U3.2 缺 VITE_API_BASE_URL 时 import 抛 [CONFIG ERROR]', async () => {
    stubAll({ VITE_API_BASE_URL: '' });
    await expect(import('./env')).rejects.toThrow(
      '[CONFIG ERROR] VITE_API_BASE_URL must be set',
    );
  });

  it('U3.3 字段值为空白字符时也抛错', async () => {
    stubAll({ VITE_WS_URL: '   ' });
    await expect(import('./env')).rejects.toThrow(
      '[CONFIG ERROR] VITE_WS_URL must be set',
    );
  });

  it('U3.4 四字段全部就绪时返回包含 4 字段的对象', async () => {
    stubAll({});
    const { webEnv } = await import('./env');
    expect(webEnv).toEqual({
      apiBaseUrl: 'http://127.0.0.1:3000/api',
      wsUrl: 'ws://127.0.0.1:3000/ws',
      adminApiBaseUrl: 'http://127.0.0.1:3001/api/v1/admin',
      analyticsEndpoint: 'https://analytics-test.example.com/collect',
    });
  });

  it('U3.5 错误信息前缀严格 [CONFIG ERROR]<空格>', async () => {
    stubAll({ VITE_ANALYTICS_ENDPOINT: '' });
    await expect(import('./env')).rejects.toThrowError(
      /^\[CONFIG ERROR\] VITE_ANALYTICS_ENDPOINT must be set$/,
    );
  });

  it('U5.2 webEnv 含 4 个字段的字符串 key', async () => {
    stubAll({});
    const { webEnv } = await import('./env');
    expect(Object.keys(webEnv).sort()).toEqual([
      'adminApiBaseUrl',
      'analyticsEndpoint',
      'apiBaseUrl',
      'wsUrl',
    ]);
    // 类型层面的只读契约由 `as const` 保证；运行时不可枚举写校验
    expect(typeof webEnv.adminApiBaseUrl).toBe('string');
  });
});
