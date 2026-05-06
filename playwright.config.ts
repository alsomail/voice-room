import { defineConfig, devices } from '@playwright/test';
import * as path from 'node:path';

/**
 * Voice Room E2E Playwright 配置（T-0000H 改造）
 *
 * 关键变化：
 *   - 不再在 config 顶层加载 dotenv；env 加载完全交由 globalSetup（envLoader）
 *   - 新增 globalSetup / globalTeardown 指向 tests/scripts/support/
 *   - profile=prod 时 grep '@prod-safe'（与 fixture L3 双保险）
 *   - use.baseURL = lazy 读 process.env.ADMIN_WEB_URL（globalSetup Step4 注入）
 *   - 单元测试见 playwright.unit.config.ts
 */
export default defineConfig({
  testDir: './tests/scripts',
  // 排除 support/__tests__（单元测试由 playwright.unit.config.ts 单独跑）
  // 铁律 7（2026-04-30）：E2E 框架统一为 Midscene；显式忽略遗留 Maestro yaml
  // 防止误调度（Playwright 默认只识别 .spec.ts，但显式声明可阻挡未来误改）。
  testIgnore: ['**/support/__tests__/**', '**/*.yaml', '**/*.yml'],

  timeout: 120 * 1000,
  expect: { timeout: 30000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : 2,

  globalSetup: path.resolve(__dirname, 'tests/scripts/support/globalSetup.ts'),
  globalTeardown: path.resolve(__dirname, 'tests/scripts/support/globalTeardown.ts'),

  // profile=prod 时强制 grep @prod-safe（与 fixture L3 双保险）
  grep: process.env.E2E_PROFILE === 'prod' ? /@prod-safe/ : undefined,

  reporter: [
    ['list'],
    ['@midscene/web/playwright-reporter', { type: 'merged' }],
    ['html', { open: 'never' }],
  ],

  use: {
    // T-0000J §2.3 修复矩阵：use.baseURL 双 key fallback。
    //   - _E2E_RUNTIME_ADMIN_WEB_URL：globalSetup Step4 writeProcessEnv 注入的 runtime 私有 key
    //   - ADMIN_WEB_URL：shell 预 export 兜底（与 T-0000F 根 .env 字段一致）
    // 求值时序：Playwright defineConfig 同步读取，但 globalSetup 在 worker `test()` 之前完成 Step4，
    // 因此 worker 侧拿到的 process.env 已写入；config 顶层求值则读 shell 预 export 值。
    baseURL: process.env._E2E_RUNTIME_ADMIN_WEB_URL ?? process.env.ADMIN_WEB_URL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  projects: [
    // Web / Admin 端 测试 —— 三个浏览器，排除 AND 目录（Android 端无需浏览器上下文）
    { name: 'chromium', testIgnore: ['**/AND/**'], use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox',  testIgnore: ['**/AND/**'], use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit',   testIgnore: ['**/AND/**'], use: { ...devices['Desktop Safari'] } },
    // Android Midscene 测试 —— 无浏览器，仅跟踪 AND/ 目录
    { name: 'android',  testMatch: ['**/AND/**'] },
  ],
});
