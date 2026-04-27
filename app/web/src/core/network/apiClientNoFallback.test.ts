/**
 * T-20020 U4.* apiClient 默认值删除契约
 *
 * 仅 grep 文件源码：
 *   - U4.1 apiClient.ts 不再硬编码 'localhost:3001/api/v1/admin'
 *   - U4.2 apiClient.ts 不再 `?? 'http...'` 兜底
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const SRC = readFileSync(
  resolve(__dirname, 'apiClient.ts'),
  'utf8',
);

describe('apiClient 默认值删除（T-20020 U4.*）', () => {
  it('U4.1 不含硬编码 localhost:3001/api/v1/admin', () => {
    expect(SRC).not.toMatch(/localhost:3001\/api\/v1\/admin/);
  });

  it("U4.2 不含 `?? 'http` 默认值兜底", () => {
    expect(SRC).not.toMatch(/\?\?\s*['"]http/);
  });

  it('U4.3 改为通过 webEnv.adminApiBaseUrl 获取', () => {
    expect(SRC).toMatch(/webEnv\.adminApiBaseUrl/);
  });
});
