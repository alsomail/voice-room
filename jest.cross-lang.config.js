/** @type {import('jest').Config} */
module.exports = {
  displayName: 'cross-lang-ws',
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/tests/cross-lang/android-server-ws/**/*.spec.ts'],
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: 'tsconfig.cross-lang.json',
      },
    ],
  },
  moduleFileExtensions: ['ts', 'js', 'json'],
  // 单测文件串行执行（WS 连接存在状态依赖，不做并发）
  maxWorkers: 1,
  // 每个测试文件最长运行时间（8 场景各有网络 I/O）
  testTimeout: 60000,
  // 不做覆盖率门禁（E2E 集成测试）
  collectCoverage: false,
  // 优雅地显示 SKIP-KNOWN 日志
  verbose: true,
};
