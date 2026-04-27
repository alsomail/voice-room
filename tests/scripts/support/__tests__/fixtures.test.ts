/**
 * fixtures 单元测试（T-0000H §2.6）
 * 不启动浏览器；通过模拟 fixture 内 testInfo.skip / playwright.request.newContext 的最小契约。
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import {
  prodSafeGuardImpl,
  apiWriteRequestSkipImpl,
} from '../fixtures';
import type { E2EEnv } from '../types';

function envOf(profile: 'local' | 'staging' | 'prod', allowWrites: boolean): E2EEnv {
  return Object.freeze({
    profile,
    allowWrites,
    appServerBaseUrl: 'http://x',
    adminServerBaseUrl: 'http://x',
    adminWebUrl: 'http://x',
    appWsUrl: 'ws://x',
    tokens: { valid: 't', expired: 't', admin: 't', op: 't', cs: 't', fin: 't', expiredAdmin: 't' },
    ids: { roomId: 'r', userAId: 'a', userBId: 'b' },
    midscene: { apiKey: '', modelName: 'gpt-4o', cache: true },
    ciReady: false,
  }) as E2EEnv;
}

test('prodSafeGuardImpl: prod + 无 @prod-safe → skip', () => {
  const skips: string[] = [];
  const ti = { tags: [], skip: (cond: boolean, reason: string) => { if (cond) skips.push(reason); } };
  prodSafeGuardImpl(envOf('prod', false), ti as any);
  expect(skips).toHaveLength(1);
  expect(skips[0]).toContain('@prod-safe');
});

test('prodSafeGuardImpl: prod + @prod-safe → 不 skip', () => {
  const skips: string[] = [];
  const ti = { tags: ['@prod-safe'], skip: (cond: boolean, reason: string) => { if (cond) skips.push(reason); } };
  prodSafeGuardImpl(envOf('prod', false), ti as any);
  expect(skips).toHaveLength(0);
});

test('prodSafeGuardImpl: local → 永不 skip', () => {
  const skips: string[] = [];
  const ti = { tags: [], skip: (cond: boolean, r: string) => { if (cond) skips.push(r); } };
  prodSafeGuardImpl(envOf('local', true), ti as any);
  expect(skips).toHaveLength(0);
});

test('apiWriteRequestSkipImpl: allowWrites=0 → skip', () => {
  const skips: string[] = [];
  const ti = { skip: (cond: boolean, r: string) => { if (cond) skips.push(r); } };
  const result = apiWriteRequestSkipImpl(envOf('prod', false), ti as any);
  expect(result).toBe(true);
  expect(skips[0]).toContain('E2E_ALLOW_WRITES');
});

test('apiWriteRequestSkipImpl: allowWrites=1 → 不 skip', () => {
  const skips: string[] = [];
  const ti = { skip: (cond: boolean, r: string) => { if (cond) skips.push(r); } };
  const result = apiWriteRequestSkipImpl(envOf('local', true), ti as any);
  expect(result).toBe(false);
  expect(skips).toHaveLength(0);
});
