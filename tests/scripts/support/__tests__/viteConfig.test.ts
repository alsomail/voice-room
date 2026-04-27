/**
 * 缺陷 3 修复（batch-e2e-foundation-01 第 1 轮）：
 *   验证 `app/web/vite.config.ts` 把根 `tests/scripts/env/.env.<profile>` 作为
 *   单一事实源注入到 `import.meta.env.VITE_*`。
 *
 *   静态体检：
 *     - vite.config.ts 必须 import loadEnv（或自实现解析）+ 指向 ../../tests/scripts/env
 *     - 必须 define 注入 VITE_ADMIN_API_BASE_URL / VITE_API_BASE_URL / VITE_WS_URL
 *     - app/web/.env.{development,staging,production,test} 不得含 https? URL 字面值
 *
 *   动态体检：
 *     - 通过 import 调用 vite.config 工厂，传 mode='staging'，断言 define 中
 *       `import.meta.env.VITE_ADMIN_API_BASE_URL` 等于 .env.staging.example 的
 *       `ADMIN_SERVER_BASE_URL` + `/api/v1/admin`。
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

const REPO_ROOT = path.resolve(__dirname, '../../../..');
const VITE_CONFIG = path.join(REPO_ROOT, 'app/web/vite.config.ts');
const ROOT_ENV_DIR = path.join(REPO_ROOT, 'tests/scripts/env');

test('U-V1 vite.config.ts 必须从根 tests/scripts/env 读 env（envDir / 自实现 dotenv 二选一）', () => {
  const text = fs.readFileSync(VITE_CONFIG, 'utf8');
  // 引用了根 env 路径
  expect(/tests\/scripts\/env/.test(text), 'vite.config.ts 未引用根 tests/scripts/env').toBe(true);
  // 必须 define 注入 VITE_*
  expect(text).toContain('import.meta.env.VITE_ADMIN_API_BASE_URL');
  expect(text).toContain('import.meta.env.VITE_API_BASE_URL');
  expect(text).toContain('import.meta.env.VITE_WS_URL');
});

test('U-V2 app/web/.env.{development,staging,production,test} 不得含字面 https?:// URL', () => {
  for (const file of ['.env.development', '.env.staging', '.env.production', '.env.test']) {
    const p = path.join(REPO_ROOT, 'app/web', file);
    if (!fs.existsSync(p)) continue;
    const text = fs.readFileSync(p, 'utf8');
    // 排除注释行后扫描
    const code = text
      .split('\n')
      .filter(l => !l.trim().startsWith('#'))
      .join('\n');
    const found = code.match(/https?:\/\/[\w.-]+/g);
    expect(found, `${file} 仍含字面 URL: ${(found || []).join(',')}`).toBeFalsy();
  }
});

test('U-V3 vite.config 工厂在 mode=staging 下注入的 VITE_ADMIN_API_BASE_URL 等于根 .env.staging.example 派生值', async () => {
  // 动态导入 vite.config（ts-node 通过 playwright 运行器编译）
  const mod = await import(VITE_CONFIG);
  const factory = mod.default;
  expect(typeof factory).toBe('function');

  // vite defineConfig 返回的是 ConfigFn 或 UserConfig；这里我们用工厂签名调
  const cfg = factory({ mode: 'staging', command: 'build' as const });
  const def = cfg.define ?? {};

  // 读 .env.staging.example 真值
  const exampleText = fs.readFileSync(path.join(ROOT_ENV_DIR, '.env.staging.example'), 'utf8');
  const m = exampleText.match(/^ADMIN_SERVER_BASE_URL=(.+)$/m);
  expect(m, '根 .env.staging.example 未声明 ADMIN_SERVER_BASE_URL').toBeTruthy();
  const adminBase = m![1].trim();
  const expected = `${adminBase}/api/v1/admin`;

  // define 值是 JSON.stringify 后的字符串（含双引号）
  const got = def['import.meta.env.VITE_ADMIN_API_BASE_URL'] as string;
  expect(got).toBe(JSON.stringify(expected));

  // VITE_API_BASE_URL 应来自 APP_SERVER_BASE_URL + /api
  const m2 = exampleText.match(/^APP_SERVER_BASE_URL=(.+)$/m);
  if (m2) {
    expect(def['import.meta.env.VITE_API_BASE_URL']).toBe(JSON.stringify(`${m2[1].trim()}/api`));
  }
});
