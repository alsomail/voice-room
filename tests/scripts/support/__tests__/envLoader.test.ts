/**
 * envLoader 单元测试（T-0000H §2.8.1）
 *
 * Runner: @playwright/test（避免引入 vitest）。
 * 测试通过准备临时仓库根（tmpdir）下的 `tests/scripts/env/.env.<profile>` 文件 +
 *   显式控制 process.env 来驱动 envLoader，行为等价于 fixture 注入。
 *
 * 红线：单测不真正调用 spawn / 不联网，profile=prod+writes=1 仅断言 console.warn 被调用。
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import {
  loadE2EEnv,
  MissingEnvError,
  InvalidProfileError,
  InvalidEnvError,
  EX_CONFIG,
} from '../envLoader';
import type { E2EEnv } from '../types';

// 24 字段 local 全量样本（值任意，只要类型对）
const LOCAL_FULL: Record<string, string> = {
  E2E_PROFILE: 'local',
  E2E_ALLOW_WRITES: '1',
  APP_SERVER_BASE_URL: 'http://localhost:3000',
  ADMIN_SERVER_BASE_URL: 'http://localhost:3001',
  ADMIN_WEB_URL: 'http://localhost:5173',
  APP_WS_URL: 'ws://localhost:3000/ws',
  DATABASE_URL: 'postgres://u:p@localhost:5432/voice_room',
  REDIS_URL: 'redis://localhost:6379',
  ANDROID_APP_ID: 'com.voiceroom.local',
  E2E_VALID_TOKEN: 'tok-valid',
  E2E_EXPIRED_TOKEN: 'tok-expired',
  E2E_ADMIN_TOKEN: 'tok-admin',
  E2E_OP_TOKEN: 'tok-op',
  E2E_CS_TOKEN: 'tok-cs',
  E2E_FIN_TOKEN: 'tok-fin',
  E2E_EXPIRED_ADMIN_TOKEN: 'tok-expired-admin',
  E2E_ROOM_ID: 'room-1',
  E2E_USER_A_ID: 'ua',
  E2E_USER_B_ID: 'ub',
  MIDSCENE_MODEL_API_KEY: '',
  MIDSCENE_MODEL_NAME: 'gpt-4o',
  MIDSCENE_OPENAI_BASE_URL: '',
  MIDSCENE_CACHE: '1',
  CI_E2E_READY: '0',
};

const STAGING_FULL: Record<string, string> = {
  ...LOCAL_FULL,
  E2E_PROFILE: 'staging',
  APP_SERVER_BASE_URL: 'https://stg-app.example.com',
  ADMIN_SERVER_BASE_URL: 'https://stg-admin.example.com',
  ADMIN_WEB_URL: 'https://stg-web.example.com',
  APP_WS_URL: 'wss://stg-app.example.com/ws',
  DATABASE_URL: '',
  REDIS_URL: '',
};

const PROD_FULL: Record<string, string> = {
  ...STAGING_FULL,
  E2E_PROFILE: 'prod',
  E2E_ALLOW_WRITES: '0',
};

/** E2E 字段名集合 — 用于在每个用例前清掉 process.env 防串扰 */
const ALL_KEYS = Object.keys(LOCAL_FULL);

/** 准备一个临时 cwd，写入 .env.<profile> 文件 */
function setupTmpCwd(profileFile: string, content: Record<string, string>): string {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'envloader-'));
  fs.mkdirSync(path.join(tmp, 'tests', 'scripts', 'env'), { recursive: true });
  const lines = Object.entries(content).map(([k, v]) => `${k}=${v}`).join('\n');
  fs.writeFileSync(path.join(tmp, 'tests', 'scripts', 'env', profileFile), lines);
  return tmp;
}

function snapshotEnv(): NodeJS.ProcessEnv {
  return { ...process.env };
}
function restoreEnv(snap: NodeJS.ProcessEnv) {
  for (const k of ALL_KEYS) delete process.env[k];
  for (const [k, v] of Object.entries(snap)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
}

test.describe('envLoader: 正向加载', () => {
  let snap: NodeJS.ProcessEnv;
  test.beforeEach(() => { snap = snapshotEnv(); for (const k of ALL_KEYS) delete process.env[k]; });
  test.afterEach(() => restoreEnv(snap));

  test('local 全字段齐全 → 返回 frozen E2EEnv', () => {
    const cwd = setupTmpCwd('.env.local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const env = loadE2EEnv({ cwd });
    expect(env.profile).toBe('local');
    expect(env.allowWrites).toBe(true);
    expect(env.appServerBaseUrl).toBe('http://localhost:3000');
    expect(env.databaseUrl).toBe('postgres://u:p@localhost:5432/voice_room');
    expect(env.tokens.valid).toBe('tok-valid');
    expect(env.ids.roomId).toBe('room-1');
    expect(env.midscene.modelName).toBe('gpt-4o');
    expect(env.midscene.cache).toBe(true);
    expect(env.ciReady).toBe(false);
    expect(Object.isFrozen(env)).toBe(true);
  });

  test('MIDSCENE_CACHE 缺省 → midscene.cache=true', () => {
    const m = { ...LOCAL_FULL };
    delete m.MIDSCENE_CACHE;
    const cwd = setupTmpCwd('.env.local', m);
    process.env.E2E_PROFILE = 'local';
    const env = loadE2EEnv({ cwd });
    expect(env.midscene.cache).toBe(true);
  });

  test('staging 全量 → DATABASE_URL/REDIS_URL 留空通过', () => {
    const cwd = setupTmpCwd('.env.staging', STAGING_FULL);
    process.env.E2E_PROFILE = 'staging';
    const env = loadE2EEnv({ cwd });
    expect(env.profile).toBe('staging');
    expect(env.databaseUrl).toBeUndefined();
    expect(env.redisUrl).toBeUndefined();
  });

  test('shell export 优先于 .env 文件', () => {
    const cwd = setupTmpCwd('.env.local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    process.env.APP_SERVER_BASE_URL = 'http://shell-wins:9999';
    const env = loadE2EEnv({ cwd });
    expect(env.appServerBaseUrl).toBe('http://shell-wins:9999');
  });

  test('prod + allowWrites=1 仅 console.warn，不抛', () => {
    const m = { ...PROD_FULL, E2E_ALLOW_WRITES: '1' };
    const cwd = setupTmpCwd('.env.prod', m);
    process.env.E2E_PROFILE = 'prod';
    const calls: string[] = [];
    const orig = console.warn;
    console.warn = (...args: unknown[]) => { calls.push(args.map(String).join(' ')); };
    try {
      const env = loadE2EEnv({ cwd });
      expect(env.allowWrites).toBe(true);
      expect(calls.some(c => c.includes('prod') && c.toLowerCase().includes('writes'))).toBe(true);
    } finally {
      console.warn = orig;
    }
  });

  test('local 默认 allowWrites=1（未设字段时）', () => {
    const m = { ...LOCAL_FULL };
    delete m.E2E_ALLOW_WRITES;
    const cwd = setupTmpCwd('.env.local', m);
    process.env.E2E_PROFILE = 'local';
    const env = loadE2EEnv({ cwd });
    expect(env.allowWrites).toBe(true);
  });

  test('prod 默认 allowWrites=0', () => {
    const m = { ...PROD_FULL };
    delete m.E2E_ALLOW_WRITES;
    const cwd = setupTmpCwd('.env.prod', m);
    process.env.E2E_PROFILE = 'prod';
    const env = loadE2EEnv({ cwd });
    expect(env.allowWrites).toBe(false);
  });
});

test.describe('envLoader: 异常分支', () => {
  let snap: NodeJS.ProcessEnv;
  test.beforeEach(() => { snap = snapshotEnv(); for (const k of ALL_KEYS) delete process.env[k]; });
  test.afterEach(() => restoreEnv(snap));

  test('staging 缺 APP_SERVER_BASE_URL → MissingEnvError', () => {
    const m = { ...STAGING_FULL };
    delete m.APP_SERVER_BASE_URL;
    const cwd = setupTmpCwd('.env.staging', m);
    process.env.E2E_PROFILE = 'staging';
    let caught: unknown;
    try { loadE2EEnv({ cwd }); } catch (e) { caught = e; }
    expect(caught).toBeInstanceOf(MissingEnvError);
    const err = caught as MissingEnvError;
    expect(err.missingFields).toContain('APP_SERVER_BASE_URL');
    expect(err.exitCode).toBe(EX_CONFIG);
    expect(err.message).toContain('Hint: copy');
    expect(err.message).toContain('.env.staging.example');
    expect(err.message).toContain('Reference: doc/tds/infra/T-0000F.md');
  });

  test('prod 缺 E2E_ADMIN_TOKEN → MissingEnvError', () => {
    const m = { ...PROD_FULL };
    delete m.E2E_ADMIN_TOKEN;
    const cwd = setupTmpCwd('.env.prod', m);
    process.env.E2E_PROFILE = 'prod';
    expect(() => loadE2EEnv({ cwd })).toThrow(MissingEnvError);
  });

  test('local 缺 DATABASE_URL → MissingEnvError', () => {
    const m = { ...LOCAL_FULL };
    delete m.DATABASE_URL;
    const cwd = setupTmpCwd('.env.local', m);
    process.env.E2E_PROFILE = 'local';
    expect(() => loadE2EEnv({ cwd })).toThrow(/DATABASE_URL/);
  });

  test('staging 缺 DATABASE_URL → 不抛（远端不校验直连）', () => {
    const m = { ...STAGING_FULL };
    delete m.DATABASE_URL;
    const cwd = setupTmpCwd('.env.staging', m);
    process.env.E2E_PROFILE = 'staging';
    expect(() => loadE2EEnv({ cwd })).not.toThrow();
  });

  test('E2E_PROFILE=invalid → InvalidProfileError, exit 78', () => {
    const cwd = setupTmpCwd('.env.local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'pre-prod';
    let caught: unknown;
    try { loadE2EEnv({ cwd }); } catch (e) { caught = e; }
    expect(caught).toBeInstanceOf(InvalidProfileError);
    expect(caught).toBeInstanceOf(MissingEnvError);
    expect((caught as InvalidProfileError).exitCode).toBe(78);
  });

  test('E2E_ALLOW_WRITES=maybe → InvalidEnvError', () => {
    const m = { ...LOCAL_FULL, E2E_ALLOW_WRITES: 'maybe' };
    const cwd = setupTmpCwd('.env.local', m);
    process.env.E2E_PROFILE = 'local';
    let caught: unknown;
    try { loadE2EEnv({ cwd }); } catch (e) { caught = e; }
    expect(caught).toBeInstanceOf(InvalidEnvError);
    expect((caught as InvalidEnvError).exitCode).toBe(78);
  });

  test('APP_SERVER_BASE_URL=not-a-url → InvalidEnvError', () => {
    const m = { ...LOCAL_FULL, APP_SERVER_BASE_URL: 'not-a-url' };
    const cwd = setupTmpCwd('.env.local', m);
    process.env.E2E_PROFILE = 'local';
    expect(() => loadE2EEnv({ cwd })).toThrow(InvalidEnvError);
  });

  test('MissingEnvError.format 格式稳定（含 Hint + Reference）', () => {
    const msg = MissingEnvError.format('staging', ['APP_SERVER_BASE_URL', 'APP_WS_URL'], '/x/.env.staging');
    expect(msg).toContain('profile=staging');
    expect(msg).toContain('missing 2 required field(s)');
    expect(msg).toContain('  - APP_SERVER_BASE_URL');
    expect(msg).toContain('  - APP_WS_URL');
    expect(msg).toContain('Hint: copy /x/.env.staging.example to /x/.env.staging');
    expect(msg).toContain('Reference: doc/tds/infra/T-0000F.md §2.3 field table');
  });

  test('默认 profile=local（process.env.E2E_PROFILE 缺省）', () => {
    const cwd = setupTmpCwd('.env.local', LOCAL_FULL);
    delete process.env.E2E_PROFILE;
    const env = loadE2EEnv({ cwd });
    expect(env.profile).toBe('local');
  });
});
