import { defineConfig } from '@playwright/test';

/**
 * T-0000H 单元测试专用 config：
 *   - 不启用 globalSetup / globalTeardown（单测自带 mock）
 *   - 仅匹配 tests/scripts/support/__tests__ 下文件
 *   - 单 project，避免跨浏览器重复
 */
export default defineConfig({
  testDir: './tests/scripts/support/__tests__',
  testMatch: '**/*.test.ts',
  timeout: 30_000,
  fullyParallel: false,
  retries: 0,
  reporter: [['list']],
  projects: [{ name: 'unit' }],
});
