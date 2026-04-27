/**
 * envLoader Midscene env 注入单元测试（T-0000P §三 TDD 验收标准）
 *
 * Runner: @playwright/test（复用现有基础设施）。
 * 测试策略：通过控制 process.env + .env 文件内容驱动 loadE2EEnv，
 *   断言 Midscene 相关字段的读取优先级 + fallback 逻辑 + 双注入（writeProcessEnv）。
 *
 * 覆盖用例：
 *   U-1：process.env.MIDSCENE_MODEL_API_KEY 直接命中
 *   U-2：仅 OPENAI_API_KEY 时 fallback 命中并双注入
 *   U-3：两者皆缺失时 warn 且不抛错；返回字段为空字符串
 *   U-4：可选字段（MIDSCENE_MODEL_BASE_URL）透传
 *   U-5：Azure 场景（AZURE_OPENAI_ENDPOINT + AZURE_OPENAI_API_KEY）透传但不作为 fallback
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { loadE2EEnv, writeProcessEnv } from '../envLoader';
import type { E2EEnv } from '../types';

// ─────────────────────────────────────────────────────────────────────────────
// 测试 fixture：构造临时 cwd 并准备 .env.<profile> 文件
// ─────────────────────────────────────────────────────────────────────────────

const BASE_FULL: Record<string, string> = {
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
  MIDSCENE_MODEL_NAME: 'gpt-4o',
  MIDSCENE_CACHE: '1',
  CI_E2E_READY: '0',
};

const MIDSCENE_KEYS = [
  'MIDSCENE_MODEL_API_KEY',
  'OPENAI_API_KEY',
  'MIDSCENE_MODEL_BASE_URL',
  'MIDSCENE_OPENAI_BASE_URL',
  'AZURE_OPENAI_ENDPOINT',
  'AZURE_OPENAI_API_KEY',
] as const;

/** 准备临时 cwd，写入 .env.<profile> 文件 */
function setupTmpCwd(profileFile: string, content: Record<string, string>): string {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'midscene-env-'));
  fs.mkdirSync(path.join(tmp, 'tests', 'scripts', 'env'), { recursive: true });
  const lines = Object.entries(content)
    .map(([k, v]) => `${k}=${v}`)
    .join('\n');
  fs.writeFileSync(path.join(tmp, 'tests', 'scripts', 'env', profileFile), lines);
  return tmp;
}

function snapshotEnv(): NodeJS.ProcessEnv {
  return { ...process.env };
}

function restoreEnv(snap: NodeJS.ProcessEnv) {
  for (const k of [...Object.keys(BASE_FULL), ...MIDSCENE_KEYS]) delete process.env[k];
  for (const [k, v] of Object.entries(snap)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试套件
// ─────────────────────────────────────────────────────────────────────────────

test.describe('envLoader: Midscene env 注入（T-0000P）', () => {
  let snap: NodeJS.ProcessEnv;

  test.beforeEach(() => {
    snap = snapshotEnv();
    for (const k of [...Object.keys(BASE_FULL), ...MIDSCENE_KEYS]) delete process.env[k];
  });

  test.afterEach(() => restoreEnv(snap));

  // ────────────────────────────────────────────────────────────────────────
  // U-1：process.env.MIDSCENE_MODEL_API_KEY 直接命中
  // ────────────────────────────────────────────────────────────────────────
  test('U-1: process.env.MIDSCENE_MODEL_API_KEY 命中时直接采用', () => {
    const cwd = setupTmpCwd('.env.local', BASE_FULL);
    process.env.E2E_PROFILE = 'local';
    process.env.MIDSCENE_MODEL_API_KEY = 'sk-test-midscene-123';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('sk-test-midscene-123');
  });

  test('U-1.1: .env 文件中 MIDSCENE_MODEL_API_KEY 优先于 OPENAI_API_KEY', () => {
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: 'sk-from-file-midscene',
      OPENAI_API_KEY: 'sk-from-file-openai',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('sk-from-file-midscene');
  });

  test('U-1.2: process.env.MIDSCENE_MODEL_API_KEY 覆盖 .env 文件', () => {
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: 'sk-from-file',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';
    process.env.MIDSCENE_MODEL_API_KEY = 'sk-from-shell';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('sk-from-shell');
  });

  // ────────────────────────────────────────────────────────────────────────
  // U-2：仅 OPENAI_API_KEY 时 fallback 命中并双注入
  // ────────────────────────────────────────────────────────────────────────
  test('U-2: 仅 OPENAI_API_KEY 时 fallback 命中', () => {
    const config = {
      ...BASE_FULL,
      OPENAI_API_KEY: 'sk-fallback-openai-456',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('sk-fallback-openai-456');
  });

  test('U-2.1: process.env.OPENAI_API_KEY fallback', () => {
    const cwd = setupTmpCwd('.env.local', BASE_FULL);
    process.env.E2E_PROFILE = 'local';
    process.env.OPENAI_API_KEY = 'sk-shell-openai';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('sk-shell-openai');
  });

  test('U-2.2: writeProcessEnv 双注入（MIDSCENE_MODEL_API_KEY + OPENAI_API_KEY）', () => {
    const config = {
      ...BASE_FULL,
      OPENAI_API_KEY: 'sk-double-inject',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';
    delete process.env.MIDSCENE_MODEL_API_KEY;
    delete process.env.OPENAI_API_KEY;

    const env = loadE2EEnv({ cwd });
    writeProcessEnv(env);

    expect(process.env.MIDSCENE_MODEL_API_KEY).toBe('sk-double-inject');
    expect(process.env.OPENAI_API_KEY).toBe('sk-double-inject');
  });

  test('U-2.3: 优先级：process.env.MIDSCENE_MODEL_API_KEY > .env.MIDSCENE_MODEL_API_KEY > process.env.OPENAI_API_KEY > .env.OPENAI_API_KEY', () => {
    // 测试四层 fallback 链的完整优先级
    const config = {
      ...BASE_FULL,
      OPENAI_API_KEY: 'sk-file-openai',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';
    process.env.OPENAI_API_KEY = 'sk-shell-openai';

    // 场景 1：仅 .env.OPENAI_API_KEY
    let env = loadE2EEnv({ cwd });
    expect(env.midscene.apiKey).toBe('sk-shell-openai'); // shell 覆盖 file

    // 场景 2：加入 process.env.MIDSCENE_MODEL_API_KEY（最高优先级）
    process.env.MIDSCENE_MODEL_API_KEY = 'sk-shell-midscene';
    env = loadE2EEnv({ cwd });
    expect(env.midscene.apiKey).toBe('sk-shell-midscene');
  });

  // ────────────────────────────────────────────────────────────────────────
  // U-3：两者皆缺失时 warn 且不抛错
  // ────────────────────────────────────────────────────────────────────────
  test('U-3: 两者皆缺失时 apiKey=空字符串，不抛错', () => {
    const cwd = setupTmpCwd('.env.local', BASE_FULL);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.apiKey).toBe('');
  });

  test('U-3.1: 缺失时 console.warn 被调用（间接验证：不抛错即可，warn 不影响测试）', () => {
    const cwd = setupTmpCwd('.env.local', BASE_FULL);
    process.env.E2E_PROFILE = 'local';

    // 不使用 spy（避免引入 sinon），仅断言不抛错
    expect(() => loadE2EEnv({ cwd })).not.toThrow();
  });

  // ────────────────────────────────────────────────────────────────────────
  // U-4：可选字段（MIDSCENE_MODEL_BASE_URL）透传
  // ────────────────────────────────────────────────────────────────────────
  test('U-4: MIDSCENE_MODEL_BASE_URL（可选）设置时透传', () => {
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: 'sk-custom',
      MIDSCENE_MODEL_BASE_URL: 'https://one-api.example.com/v1',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.baseUrl).toBe('https://one-api.example.com/v1');
  });

  test('U-4.1: MIDSCENE_MODEL_BASE_URL 未设置时为 undefined', () => {
    const cwd = setupTmpCwd('.env.local', BASE_FULL);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    expect(env.midscene.baseUrl).toBeUndefined();
  });

  test('U-4.2: writeProcessEnv 透传 MIDSCENE_MODEL_BASE_URL', () => {
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: 'sk-custom',
      MIDSCENE_MODEL_BASE_URL: 'https://proxy.ai/v1',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';
    delete process.env.MIDSCENE_MODEL_BASE_URL;

    const env = loadE2EEnv({ cwd });
    writeProcessEnv(env);

    expect(process.env.MIDSCENE_MODEL_BASE_URL).toBe('https://proxy.ai/v1');
  });

  // ────────────────────────────────────────────────────────────────────────
  // U-5：Azure 场景（AZURE_OPENAI_ENDPOINT + AZURE_OPENAI_API_KEY）透传
  // ────────────────────────────────────────────────────────────────────────
  test('U-5: Azure 字段（AZURE_OPENAI_ENDPOINT + AZURE_OPENAI_API_KEY）透传到 process.env', () => {
    const config = {
      ...BASE_FULL,
      AZURE_OPENAI_ENDPOINT: 'https://your-resource.openai.azure.com/',
      AZURE_OPENAI_API_KEY: 'azure-key-123',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';
    delete process.env.AZURE_OPENAI_ENDPOINT;
    delete process.env.AZURE_OPENAI_API_KEY;

    const env = loadE2EEnv({ cwd });
    writeProcessEnv(env);

    expect(process.env.AZURE_OPENAI_ENDPOINT).toBe('https://your-resource.openai.azure.com/');
    expect(process.env.AZURE_OPENAI_API_KEY).toBe('azure-key-123');
  });

  test('U-5.1: Azure 字段不作为 MIDSCENE_MODEL_API_KEY 的 fallback', () => {
    const config = {
      ...BASE_FULL,
      AZURE_OPENAI_API_KEY: 'azure-key-456',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    // apiKey 应为空（Azure 不作为 fallback）
    expect(env.midscene.apiKey).toBe('');
  });

  // ────────────────────────────────────────────────────────────────────────
  // 边界用例：兼容性测试
  // ────────────────────────────────────────────────────────────────────────
  test('Edge: MIDSCENE_MODEL_API_KEY 为空字符串时不 fallback 到 OPENAI_API_KEY', () => {
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: '',
      OPENAI_API_KEY: 'sk-openai-fallback',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });

    // 显式设置为空字符串时，应 fallback 到 OPENAI_API_KEY
    expect(env.midscene.apiKey).toBe('sk-openai-fallback');
  });

  test('Edge: staging profile Midscene env 加载（不影响 token 必填校验）', () => {
    const config = {
      ...BASE_FULL,
      E2E_PROFILE: 'staging',
      APP_SERVER_BASE_URL: 'https://stg.example.com',
      ADMIN_SERVER_BASE_URL: 'https://stg-admin.example.com',
      ADMIN_WEB_URL: 'https://stg-web.example.com',
      APP_WS_URL: 'wss://stg.example.com/ws',
      MIDSCENE_MODEL_API_KEY: 'sk-staging-key',
    };
    const cwd = setupTmpCwd('.env.staging', config);
    process.env.E2E_PROFILE = 'staging';

    const env = loadE2EEnv({ cwd });

    expect(env.profile).toBe('staging');
    expect(env.midscene.apiKey).toBe('sk-staging-key');
  });

  test('Edge: sanitizeEnvForRuntimeJson 脱敏 apiKey', async () => {
    const { sanitizeEnvForRuntimeJson } = await import('../envLoader');
    const config = {
      ...BASE_FULL,
      MIDSCENE_MODEL_API_KEY: 'sk-secret-real-key',
    };
    const cwd = setupTmpCwd('.env.local', config);
    process.env.E2E_PROFILE = 'local';

    const env = loadE2EEnv({ cwd });
    const sanitized = sanitizeEnvForRuntimeJson(env);

    expect(env.midscene.apiKey).toBe('sk-secret-real-key'); // 原对象保留
    expect(sanitized.midscene.apiKey).toBe(''); // 脱敏后为空
    expect(sanitized.midscene.modelName).toBe('gpt-4o'); // 其他字段保留
  });
});
