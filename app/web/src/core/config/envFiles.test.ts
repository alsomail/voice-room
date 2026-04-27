/**
 * T-20020 U2.* 字段冻结 / U5.1 文件清单 / U6.* 跨端锚点（自动化部分）
 *
 * 通过文件系统读取 5 档 .env 文件 + vite-env.d.ts + tests/scripts/env/.env.local.example，
 * 断言字段名集合一致、host:port 锚定一致。
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const WEB_ROOT = resolve(__dirname, '../../..');
const REPO_ROOT = resolve(WEB_ROOT, '../..');

const REQUIRED_FIELDS = [
  'VITE_API_BASE_URL',
  'VITE_WS_URL',
  'VITE_ADMIN_API_BASE_URL',
  'VITE_ANALYTICS_ENDPOINT',
] as const;

function readEnvKeys(p: string): string[] {
  const txt = readFileSync(p, 'utf8');
  const keys: string[] = [];
  for (const line of txt.split(/\r?\n/)) {
    const m = line.match(/^([A-Z0-9_]+)\s*=/);
    if (m && m[1].startsWith('VITE_')) keys.push(m[1]);
  }
  return keys.sort();
}

function readEnvMap(p: string): Record<string, string> {
  const txt = readFileSync(p, 'utf8');
  const map: Record<string, string> = {};
  for (const line of txt.split(/\r?\n/)) {
    const m = line.match(/^([A-Z0-9_]+)\s*=\s*(.*)$/);
    if (m) map[m[1]] = m[2].trim();
  }
  return map;
}

describe('Web env 字段冻结契约（T-20020 U2.* / U5.* / U6.*）', () => {
  it('U2.1 .env.example 字段集合 = 4 字段冻结表', () => {
    const keys = readEnvKeys(resolve(WEB_ROOT, '.env.example'));
    expect(keys).toEqual([...REQUIRED_FIELDS].sort());
  });

  it('U2.2 5 档 .env 文件字段集合完全一致（防漂移）', () => {
    const files = [
      '.env.example',
      '.env.development',
      '.env.test',
      '.env.staging',
      '.env.production',
    ];
    const sets = files.map((f) => readEnvKeys(resolve(WEB_ROOT, f)));
    const baseline = sets[0];
    for (let i = 1; i < sets.length; i++) {
      expect(sets[i], `${files[i]} 字段不一致`).toEqual(baseline);
    }
  });

  it('U2.3 vite-env.d.ts 字段集合 = 4 字段冻结表', () => {
    const dts = readFileSync(
      resolve(WEB_ROOT, 'src/vite-env.d.ts'),
      'utf8',
    );
    for (const f of REQUIRED_FIELDS) {
      expect(dts, `vite-env.d.ts 缺字段 ${f}`).toMatch(
        new RegExp(`readonly\\s+${f}\\s*:\\s*string`),
      );
    }
  });

  it('U5.1 .env.test 与 .env.staging 文件存在且 VITE_API_BASE_URL 默认同源', () => {
    const t = readEnvMap(resolve(WEB_ROOT, '.env.test'));
    const s = readEnvMap(resolve(WEB_ROOT, '.env.staging'));
    expect(t.VITE_API_BASE_URL).toBeTruthy();
    expect(s.VITE_API_BASE_URL).toBeTruthy();
    expect(t.VITE_API_BASE_URL).toBe(s.VITE_API_BASE_URL);
  });

  it('U6.2 .env.development 中 VITE_ADMIN_API_BASE_URL host:port 与 tests/scripts/env/.env.local.example ADMIN_SERVER_BASE_URL 对齐', () => {
    const dev = readEnvMap(resolve(WEB_ROOT, '.env.development'));
    const e2e = readEnvMap(
      resolve(REPO_ROOT, 'tests/scripts/env/.env.local.example'),
    );
    const devHost = new URL(dev.VITE_ADMIN_API_BASE_URL).host;
    const e2eHost = new URL(e2e.ADMIN_SERVER_BASE_URL).host;
    // 允许 127.0.0.1 ↔ localhost 等价（同 :3001）
    const norm = (h: string) => h.replace(/^localhost/, '127.0.0.1');
    expect(norm(devHost)).toBe(norm(e2eHost));
  });
});
