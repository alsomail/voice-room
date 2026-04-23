import { defineConfig, devices } from '@playwright/test';

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// 核心修改 1：取消这里的注释，让 Playwright 启动时自动读取项目根目录的 .env 文件。
// （这样 Midscene 就能顺利读到 MIDSCENE_MODEL_API_KEY 等大模型配置了）
import dotenv from 'dotenv';
import path from 'path';
dotenv.config({ path: path.resolve(__dirname, '.env') });

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './tests/scripts',
  
  /* 核心修改 2：延长全局超时时间。因为 AI 视觉大模型识别页面和推理需要时间，单条用例默认的 30 秒通常不够，建议延长到 120 秒 */
  timeout: 120 * 1000,
  expect: {
    timeout: 15000,
  },

  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  
  /* 核心修改 3：替换 Reporter。挂载 Midscene 的专属可视化合并报告 */
  reporter: [
    ['list'], // 终端输出进度
    ['@midscene/web/playwright-reporter', { type: 'merged' }], // 生成 Midscene 的 AI 视觉分析报告
    ['html', { open: 'never' }] // 保留原生 HTML 报告作为兜底备用
  ],

  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('')`. */
    // baseURL: 'http://localhost:3000',

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',
    /* 核心修改 4：失败时自动截图，配合 AI 排查 */
    screenshot: 'only-on-failure',
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },

    // 如果您只想先测试 Chrome 浏览器，可以暂时把 firefox 和 webkit 注释掉以节省时间
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },

    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },

    /* Test against mobile viewports. */
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: { ...devices['Desktop Edge'], channel: 'msedge' },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: { ...devices['Desktop Chrome'], channel: 'chrome' },
    // },
  ],

  /* Run your local dev server before starting the tests */
  // webServer: {
  //   command: 'npm run start',
  //   url: 'http://localhost:3000',
  //   reuseExistingServer: !process.env.CI,
  // },
});