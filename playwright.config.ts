import { defineConfig, devices } from '@playwright/test';
import * as path from 'node:path';

/**
 * Voice Room E2E Playwright 配置（T-0000H 改造）
 *
 * 关键变化：
 *   - 不再在 config 顶层 dotenv.config()；env 加载完全交由 globalSetup（envLoader）
 *   - 新增 globalSetup / globalTeardown 指向 tests/scripts/support/
 *   - profile=prod 时 grep '@prod-safe'（与 fixture L3 双保险）
 *   - use.baseURL = lazy 读 process.env.ADMIN_WEB_URL（globalSetup Step4 注入）
 *   - 单元测试见 playwright.unit.config.ts
 */
export default defineConfig({
  testDir: './tests/scripts',
  // 排除 support/__tests__（单元测试由 playwright.unit.config.ts 单独跑）
  testIgnore: ['**/support/__tests__/**'],

  timeout: 120 * 1000,
  expect: { timeout: 15000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,

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
    // baseURL 在 globalSetup Step4 通过 writeProcessEnv 注入；冷启动前若 shell 未 export，
    // 用户可在根 .env 设置 ADMIN_WEB_URL；T-0000J 阶段会改用 e2eEnv fixture 注入。
    baseURL: process.env.ADMIN_WEB_URL || undefined,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
