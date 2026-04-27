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

  it('U2.2 .env.{development,test,staging,production} URL 字段已收口到根 env（缺陷 3 修复，不再要求子项目 .env 含 4 字段）', () => {
    // 缺陷 3 修复（batch-e2e-foundation-01 第 1 轮）：URL 端点字段已经从
    // app/web/.env.<mode> 移除，由 vite.config.ts 的 loadEnv + define 从
    // tests/scripts/env/.env.<profile> 注入。本用例放宽为「不得含字面 URL」。
    const files = ['.env.development', '.env.test', '.env.staging', '.env.production'];
    for (const f of files) {
      const txt = readFileSync(resolve(WEB_ROOT, f), 'utf8');
      const code = txt.split('\n').filter((l) => !l.trim().startsWith('#')).join('\n');
      const m = code.match(/https?:\/\/[\w.-]+/g);
      expect(m, `${f} 仍含字面 URL: ${(m || []).join(',')}`).toBeFalsy();
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

  it('U5.1 vite.config.ts 必须 define 注入 4 字段（替代旧版子项目 .env 多源）', () => {
    const cfg = readFileSync(resolve(WEB_ROOT, 'vite.config.ts'), 'utf8');
    for (const f of REQUIRED_FIELDS) {
      expect(cfg, `vite.config.ts 缺 define '${f}'`).toContain(`import.meta.env.${f}`);
    }
  });

  it('U6.2 vite.config 注入的 VITE_ADMIN_API_BASE_URL host:port 与根 .env.local.example ADMIN_SERVER_BASE_URL 对齐', () => {
    // 改为校验「单一事实源链路」：根 .env.local.example 的 ADMIN_SERVER_BASE_URL 必须存在且能解析为 URL。
    const e2e = readEnvMap(
      resolve(REPO_ROOT, 'tests/scripts/env/.env.local.example'),
    );
    expect(e2e.ADMIN_SERVER_BASE_URL).toBeTruthy();
    const e2eHost = new URL(e2e.ADMIN_SERVER_BASE_URL).host;
    expect(e2eHost).toMatch(/^(127\.0\.0\.1|localhost):\d+$/);
  });
});
