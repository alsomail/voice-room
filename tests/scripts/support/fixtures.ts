/**
 * T-0000H Playwright fixtures：
 *   - e2eEnv (worker scope)：每 worker 加载一次 E2EEnv（优先读 .e2e-runtime.json，fallback envLoader）
 *   - prodSafeGuard (auto)：profile=prod 且未打 @prod-safe 标签的用例自动 skip
 *   - apiWriteRequest：写操作 fixture，allowWrites=0 时 skip
 *
 * 与 T-0000J 协同：用例端只需 import { test, expect } from './support/fixtures'。
 */
import { test as base, expect, type APIRequestContext, type TestInfo } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

import { loadE2EEnv } from './envLoader';
import type { E2EEnv } from './types';

// ─────────────────────────────────────────────────────────────────────────────
// 纯函数实现（便于单测）
// ─────────────────────────────────────────────────────────────────────────────

interface MinimalTestInfo {
  tags?: string[] | readonly string[];
  skip(condition: boolean, description: string): void;
}

/** L3：profile=prod 且未打 @prod-safe 标签 → skip。返回 true 表示已 skip。 */
export function prodSafeGuardImpl(env: E2EEnv, testInfo: MinimalTestInfo): boolean {
  if (env.profile !== 'prod') return false;
  const tags = testInfo.tags ?? [];
  if (!tags.includes('@prod-safe')) {
    testInfo.skip(true, 'prod profile only runs tests tagged @prod-safe');
    return true;
  }
  return false;
}

/**
 * T-0000K：Midscene 就绪检查（auto fixture 用）。
 *
 * 仅当 testInfo.file 位于 `tests/scripts/WEB/` 子树时生效（跨平台路径分隔符兼容）。
 * 当 env.midscene.apiKey 为空（且 process.env.MIDSCENE_MODEL_API_KEY 也为空）时，
 * 调用 testInfo.skip(true, '[MIDSCENE] api key missing — skipped') 并返回 true。
 *
 * 与 prodSafeGuardImpl 严格对称：纯函数、无副作用（除 testInfo.skip），易单测。
 */
export function midsceneReadyImpl(
  env: E2EEnv,
  testInfo: MinimalTestInfo & { file?: string },
): boolean {
  const file = testInfo.file ?? '';
  const sep = path.sep;
  const isWebSpec =
    file.includes(`${sep}tests${sep}scripts${sep}WEB${sep}`) ||
    file.includes('/tests/scripts/WEB/');
  if (!isWebSpec) return false;

  const fromEnv = (env.midscene?.apiKey ?? '').trim();
  const fromProcess = (process.env.MIDSCENE_MODEL_API_KEY ?? '').trim();
  if (fromEnv !== '' || fromProcess !== '') return false;

  testInfo.skip(true, '[MIDSCENE] api key missing — skipped');
  return true;
}

/** L4：写操作 fixture 在 allowWrites=0 时 skip。返回 true 表示已 skip。 */
export function apiWriteRequestSkipImpl(env: E2EEnv, testInfo: MinimalTestInfo): boolean {
  if (!env.allowWrites) {
    testInfo.skip(true, 'fixture requires E2E_ALLOW_WRITES=1');
    return true;
  }
  return false;
}

/** 读 .e2e-runtime.json；不存在则 fallback 到 envLoader。 */
export function readE2EEnvForWorker(cwd: string = process.cwd()): E2EEnv {
  const runtimePath = path.join(cwd, 'tests/scripts/.e2e-runtime.json');
  if (fs.existsSync(runtimePath)) {
    try {
      const parsed = JSON.parse(fs.readFileSync(runtimePath, 'utf8')) as E2EEnv;
      return Object.freeze(parsed);
    } catch {
      // fallthrough
    }
  }
  return loadE2EEnv({ cwd });
}

// ─────────────────────────────────────────────────────────────────────────────
// Playwright fixtures
// ─────────────────────────────────────────────────────────────────────────────

type Fixtures = {
  e2eEnv: E2EEnv;
  prodSafeGuard: void;
  midsceneReady: void;
  apiWriteRequest: APIRequestContext;
};

export const test = base.extend<Fixtures, { e2eEnvWorker: E2EEnv }>({
  e2eEnvWorker: [async ({}, use) => {
    await use(readE2EEnvForWorker());
  }, { scope: 'worker' }],

  e2eEnv: async ({ e2eEnvWorker }, use) => {
    await use(e2eEnvWorker);
  },

  prodSafeGuard: [async ({ e2eEnv }, use, testInfo) => {
    prodSafeGuardImpl(e2eEnv, testInfo as unknown as MinimalTestInfo);
    await use();
  }, { auto: true }],

  midsceneReady: [async ({ e2eEnv }, use, testInfo) => {
    midsceneReadyImpl(
      e2eEnv,
      testInfo as unknown as MinimalTestInfo & { file?: string },
    );
    await use();
  }, { auto: true }],

  apiWriteRequest: async ({ e2eEnv, playwright }, use, testInfo) => {
    if (apiWriteRequestSkipImpl(e2eEnv, testInfo as unknown as MinimalTestInfo)) {
      // skip 之后 fixture 仍需返回（Playwright 要求 use 调用）
      // 提供一个最小桩；测试已 skip 不会真正使用
      const ctx = await playwright.request.newContext({ baseURL: e2eEnv.appServerBaseUrl });
      await use(ctx);
      await ctx.dispose();
      return;
    }
    const ctx = await playwright.request.newContext({ baseURL: e2eEnv.appServerBaseUrl });
    await use(ctx);
    await ctx.dispose();
  },
});

export { expect };
