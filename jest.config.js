/** @type {import('jest').Config} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/scripts/audit/__tests__/**/*.test.ts'],
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: 'tsconfig.scripts.json',
      },
    ],
  },
  moduleFileExtensions: ['ts', 'js', 'json'],
  collectCoverageFrom: ['scripts/audit/**/*.ts', '!scripts/audit/__tests__/**'],
  coverageThreshold: {
    global: {
      branches: 80,
      functions: 80,
      lines: 80,
      statements: 80,
    },
  },
};
