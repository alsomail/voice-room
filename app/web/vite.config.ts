import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

// ESM/CJS 兼容：vitest / playwright 单测运行时 `__dirname` 可能不可用，回退到 import.meta.url。
const __filenameSafe =
  typeof __filename !== 'undefined' ? __filename : fileURLToPath(import.meta.url);
const __dirnameSafe = path.dirname(__filenameSafe);

/**
 * 缺陷 3 修复（batch-e2e-foundation-01 第 1 轮）：
 *   AdminWeb env 单一事实源 — 收口到根 `tests/scripts/env/.env.{profile}`。
 *
 *   关键映射（vite mode → profile → 文件）：
 *     - dev / development → local → tests/scripts/env/.env.local
 *     - staging           → staging → tests/scripts/env/.env.staging
 *     - prod / production → prod   → tests/scripts/env/.env.prod
 *     - test              → local（vitest 走 src/test/setup.ts stub，不读盘）
 *
 *   覆盖优先级：process.env > 根 .env.<profile> > 根 .env.<profile>.example > 内置
 *
 *   Web 独有字段（如 `VITE_ANALYTICS_ENDPOINT`）仍可在 app/web/.env.<mode> 维护；
 *   URL 端点字段已从 app/web/.env.* 移除（只保留 web 独有 K=V），见 .env.example 注释。
 */

const PROFILE_BY_MODE: Record<string, 'local' | 'staging' | 'prod'> = {
  development: 'local',
  dev: 'local',
  test: 'local',
  staging: 'staging',
  production: 'prod',
  prod: 'prod',
};

/** 自实现 dotenv 解析（避免新增依赖），只取 KEY=VALUE 行。 */
function parseDotenv(file: string): Record<string, string> {
  const out: Record<string, string> = {};
  if (!fs.existsSync(file)) return out;
  const text = fs.readFileSync(file, 'utf8');
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq <= 0) continue;
    const k = line.slice(0, eq).trim();
    let v = line.slice(eq + 1).trim();
    if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) {
      v = v.slice(1, -1);
    }
    out[k] = v;
  }
  return out;
}

export default defineConfig(({ mode }) => {
  const profile = PROFILE_BY_MODE[mode] ?? 'local';
  const rootEnvDir = path.resolve(__dirnameSafe, '../../tests/scripts/env');

  // 优先读真实 .env.<profile>，没有则 fallback 到 .example（CI / 干净 clone 用）
  const rootEnv: Record<string, string> = {
    ...parseDotenv(path.join(rootEnvDir, `.env.${profile}.example`)),
    ...parseDotenv(path.join(rootEnvDir, `.env.${profile}`)),
  };

  // 同时让 vite 加载 app/web/.env.<mode> 中残留的 web-only VITE_*（如 VITE_ANALYTICS_ENDPOINT）
  const webLocalEnv = loadEnv(mode, __dirnameSafe, ['VITE_']);

  // 从根 env 派生 VITE_*（覆盖 app/web/.env.<mode> 中可能残留的同名 URL key）
  const adminApiBaseUrl =
    process.env.VITE_ADMIN_API_BASE_URL ??
    rootEnv.VITE_ADMIN_API_BASE_URL ??
    (rootEnv.ADMIN_SERVER_BASE_URL ? `${rootEnv.ADMIN_SERVER_BASE_URL}/api/v1/admin` : '');
  const apiBaseUrl =
    process.env.VITE_API_BASE_URL ??
    rootEnv.VITE_API_BASE_URL ??
    (rootEnv.APP_SERVER_BASE_URL ? `${rootEnv.APP_SERVER_BASE_URL}/api` : '');
  const wsUrl =
    process.env.VITE_WS_URL ??
    rootEnv.VITE_WS_URL ??
    rootEnv.APP_WS_URL ??
    '';
  const analyticsEndpoint =
    process.env.VITE_ANALYTICS_ENDPOINT ??
    rootEnv.VITE_ANALYTICS_ENDPOINT ??
    webLocalEnv.VITE_ANALYTICS_ENDPOINT ??
    '';

  return {
    plugins: [react()],
    envDir: __dirnameSafe,
    define: {
      // 4 字段共享真源 → import.meta.env.VITE_*（构建期 inline）
      'import.meta.env.VITE_ADMIN_API_BASE_URL': JSON.stringify(adminApiBaseUrl),
      'import.meta.env.VITE_API_BASE_URL': JSON.stringify(apiBaseUrl),
      'import.meta.env.VITE_WS_URL': JSON.stringify(wsUrl),
      'import.meta.env.VITE_ANALYTICS_ENDPOINT': JSON.stringify(analyticsEndpoint),
    },
    test: {
      environment: 'jsdom',
      globals: true,
      setupFiles: ['./src/test/setup.ts'],
      server: {
        deps: {
          // @exodus/bytes 是纯 ESM 包，jsdom 的 html-encoding-sniffer 依赖它。
          // 使用正则匹配所有子路径（如 encoding-lite.js），避免 CJS require() 失败。
          inline: [/^@exodus\/bytes/],
        },
      },
    },
  };
});
