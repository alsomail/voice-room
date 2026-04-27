/**
 * T-0000H envLoader：E2E 多环境配置唯一加载源。
 *
 * 设计契约（与 TDS §2.3 严格对齐）：
 *   - 纯函数 + fail-fast；不写 process.env（globalSetup Step 4 单独负责）。
 *   - 加载链：process.env > tests/scripts/env/.env.<profile> > .env > 默认值。
 *   - 必填字段缺失 → MissingEnvError，进程退出码 78（EX_CONFIG）。
 *   - profile 非 local|staging|prod → InvalidProfileError（退出 78）。
 *   - 类型校验失败（bool/url）→ InvalidEnvError（退出 78）。
 *
 * 上游契约消费：T-0000F §2.3（24 字段表）+ §2.4（错误格式）。
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import dotenv from 'dotenv';

import type { E2EEnv, E2EProfile } from './types';

export const EX_CONFIG = 78 as const;

const VALID_PROFILES: readonly E2EProfile[] = ['local', 'staging', 'prod'] as const;

// ─────────────────────────────────────────────────────────────────────────────
// 错误类型
// ─────────────────────────────────────────────────────────────────────────────

export class MissingEnvError extends Error {
  public readonly exitCode = EX_CONFIG;
  constructor(
    public readonly profile: string,
    public readonly missingFields: string[],
    public readonly envFilePath: string,
  ) {
    super(MissingEnvError.format(profile, missingFields, envFilePath));
    this.name = 'MissingEnvError';
  }
  static format(profile: string, fields: string[], filePath: string): string {
    const lines = [
      `[E2E envLoader] MissingEnvError: profile=${profile} missing ${fields.length} required field(s):`,
      ...fields.map(f => `  - ${f}`),
      `Hint: copy ${filePath}.example to ${filePath} and fill in values.`,
      `Reference: doc/tds/infra/T-0000F.md §2.3 field table.`,
    ];
    return lines.join('\n');
  }
}

export class InvalidProfileError extends MissingEnvError {
  constructor(rawProfile: string) {
    super(rawProfile, ['E2E_PROFILE'], '<env>');
    this.name = 'InvalidProfileError';
    this.message =
      `[E2E envLoader] InvalidProfileError: E2E_PROFILE='${rawProfile}' is not one of ${VALID_PROFILES.join('|')}.\n` +
      `Reference: doc/tds/infra/T-0000F.md §2.3 field table.`;
  }
}

export class InvalidEnvError extends MissingEnvError {
  constructor(field: string, rawValue: string, reason: string) {
    super('?', [field], '<env>');
    this.name = 'InvalidEnvError';
    this.message =
      `[E2E envLoader] InvalidEnvError: ${field}='${rawValue}' is invalid (${reason}).\n` +
      `Reference: doc/tds/infra/T-0000F.md §2.3 field table.`;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 内部工具
// ─────────────────────────────────────────────────────────────────────────────

/** 读 .env 文件并 parse，不修改 process.env；不存在或读失败返回空对象。 */
function readDotenvFile(filePath: string): Record<string, string> {
  try {
    if (!fs.existsSync(filePath)) return {};
    const buf = fs.readFileSync(filePath);
    return dotenv.parse(buf);
  } catch {
    return {};
  }
}

/** 解析布尔；undefined 返回 default；非法值抛 InvalidEnvError。 */
function parseBool(field: string, raw: string | undefined, def: boolean): boolean {
  if (raw === undefined || raw === '') return def;
  const v = raw.trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(v)) return true;
  if (['0', 'false', 'no', 'off'].includes(v)) return false;
  throw new InvalidEnvError(field, raw, 'expected boolean (1/0/true/false/yes/no)');
}

/** 校验 URL 形式（http/https/ws/wss/postgres/redis/...）。 */
function assertValidUrl(field: string, raw: string): void {
  try {
    // eslint-disable-next-line no-new
    new URL(raw);
  } catch {
    throw new InvalidEnvError(field, raw, 'not a valid URL');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 主入口
// ─────────────────────────────────────────────────────────────────────────────

export interface LoadOpts {
  /** 仓库根目录；默认 process.cwd()。 */
  cwd?: string;
}

/**
 * 加载并校验 E2E 配置。fail-fast；不写 process.env。
 */
export function loadE2EEnv(opts: LoadOpts = {}): E2EEnv {
  const cwd = opts.cwd ?? process.cwd();

  // ── Step 1：解析 profile（process.env 优先）──
  const rawProfile = (process.env.E2E_PROFILE ?? 'local').trim();
  if (!VALID_PROFILES.includes(rawProfile as E2EProfile)) {
    throw new InvalidProfileError(rawProfile);
  }
  const profile = rawProfile as E2EProfile;

  // ── Step 2：构建合并字典（优先级 高→低：shell > .env.<profile> > .env > 默认）──
  const profileFile = path.join(cwd, 'tests', 'scripts', 'env', `.env.${profile}`);
  const rootEnvFile = path.join(cwd, '.env');
  const fromRoot = readDotenvFile(rootEnvFile);
  const fromProfile = readDotenvFile(profileFile);
  // 合并：profile 文件覆盖根 .env
  const merged: Record<string, string | undefined> = { ...fromRoot, ...fromProfile };
  // shell（process.env 已存在的非空值）覆盖文件
  const get = (key: string): string | undefined => {
    const shell = process.env[key];
    if (shell !== undefined && shell !== '') return shell;
    const fromFile = merged[key];
    if (fromFile !== undefined && fromFile !== '') return fromFile;
    return undefined;
  };

  // ── Step 3：Midscene env 读取（T-0000P §二 改造点位 1）──
  // 优先级链：process.env.MIDSCENE_MODEL_API_KEY > .env.MIDSCENE_MODEL_API_KEY
  //          > process.env.OPENAI_API_KEY > .env.OPENAI_API_KEY
  // 缺失时仅 warn（不计入必填字段，不阻塞 API/INFRA 测试）
  let midsceneApiKey = get('MIDSCENE_MODEL_API_KEY');
  if (!midsceneApiKey) {
    midsceneApiKey = get('OPENAI_API_KEY');
  }
  if (!midsceneApiKey) {
    // eslint-disable-next-line no-console
    console.warn(
      '\x1b[33m⚠️  Midscene AI key not set (MIDSCENE_MODEL_API_KEY or OPENAI_API_KEY missing).\n' +
      '   WEB E2E tests using AI assertions will be skipped.\n' +
      '   See doc/tests/MIDSCENE_SETUP.md for setup instructions.\x1b[0m'
    );
  }

  // 可选字段：MIDSCENE_MODEL_BASE_URL / AZURE 双字段（若设置则透传，不做 fallback 推导）
  const midsceneBaseUrl = get('MIDSCENE_MODEL_BASE_URL') || get('MIDSCENE_OPENAI_BASE_URL');
  const azureEndpoint = get('AZURE_OPENAI_ENDPOINT');
  const azureApiKey = get('AZURE_OPENAI_API_KEY');

  // ── Step 4：必填字段校验 ──
  const missing: string[] = [];
  const required = (key: string) => {
    const v = get(key);
    if (v === undefined) missing.push(key);
    return v;
  };

  const appServerBaseUrl = required('APP_SERVER_BASE_URL');
  const adminServerBaseUrl = required('ADMIN_SERVER_BASE_URL');
  const adminWebUrl = required('ADMIN_WEB_URL');
  const appWsUrl = required('APP_WS_URL');

  let databaseUrl: string | undefined;
  let redisUrl: string | undefined;
  if (profile === 'local') {
    databaseUrl = required('DATABASE_URL');
    redisUrl = required('REDIS_URL');
  } else {
    databaseUrl = undefined;
    redisUrl = undefined;
  }

  // Tokens & ids：staging/prod 必填；local 加载期允许空（seed 回填）
  const tokenFields = ['E2E_VALID_TOKEN', 'E2E_EXPIRED_TOKEN', 'E2E_ADMIN_TOKEN', 'E2E_OP_TOKEN', 'E2E_CS_TOKEN', 'E2E_FIN_TOKEN', 'E2E_EXPIRED_ADMIN_TOKEN'] as const;
  const idFields = ['E2E_ROOM_ID', 'E2E_USER_A_ID', 'E2E_USER_B_ID'] as const;
  const tokenVals: Record<string, string | undefined> = {};
  for (const f of tokenFields) {
    const v = get(f);
    if (profile !== 'local' && v === undefined) missing.push(f);
    tokenVals[f] = v;
  }
  const idVals: Record<string, string | undefined> = {};
  for (const f of idFields) {
    const v = get(f);
    if (profile !== 'local' && v === undefined) missing.push(f);
    idVals[f] = v;
  }

  if (missing.length > 0) {
    throw new MissingEnvError(profile, missing, profileFile);
  }

  // ── Step 5：类型 / URL 校验 ──
  for (const [field, val] of [
    ['APP_SERVER_BASE_URL', appServerBaseUrl!],
    ['ADMIN_SERVER_BASE_URL', adminServerBaseUrl!],
    ['ADMIN_WEB_URL', adminWebUrl!],
    ['APP_WS_URL', appWsUrl!],
  ] as const) {
    assertValidUrl(field, val);
  }
  if (databaseUrl) assertValidUrl('DATABASE_URL', databaseUrl);
  if (redisUrl) assertValidUrl('REDIS_URL', redisUrl);

  const allowWritesDefault = profile === 'prod' ? false : true;
  const allowWrites = parseBool('E2E_ALLOW_WRITES', get('E2E_ALLOW_WRITES'), allowWritesDefault);

  if (profile === 'prod' && allowWrites) {
    // L2 防线：不抛错，仅 warn（用户已显式同意）
    // eslint-disable-next-line no-console
    console.warn(
      '\x1b[33m[E2E envLoader] WARN: profile=prod with E2E_ALLOW_WRITES=1 — writes will hit production. ' +
      'Ensure read-only DB role + @prod-safe tag protections are in place. ' +
      'Reference: doc/tds/infra/T-0000H.md §2.6.\x1b[0m'
    );
  }

  const midsceneCache = parseBool('MIDSCENE_CACHE', get('MIDSCENE_CACHE'), true);
  const ciReady = parseBool('CI_E2E_READY', get('CI_E2E_READY'), false);

  // ── Step 6：组装 + 冻结 ──
  const env: E2EEnv = {
    profile,
    allowWrites,
    appServerBaseUrl: appServerBaseUrl!,
    adminServerBaseUrl: adminServerBaseUrl!,
    adminWebUrl: adminWebUrl!,
    appWsUrl: appWsUrl!,
    databaseUrl,
    redisUrl,
    androidAppId: get('ANDROID_APP_ID'),
    tokens: {
      valid: tokenVals.E2E_VALID_TOKEN ?? '',
      expired: tokenVals.E2E_EXPIRED_TOKEN ?? '',
      admin: tokenVals.E2E_ADMIN_TOKEN ?? '',
      op: tokenVals.E2E_OP_TOKEN ?? '',
      cs: tokenVals.E2E_CS_TOKEN ?? '',
      fin: tokenVals.E2E_FIN_TOKEN ?? '',
      expiredAdmin: tokenVals.E2E_EXPIRED_ADMIN_TOKEN ?? '',
    },
    ids: {
      roomId: idVals.E2E_ROOM_ID ?? '',
      userAId: idVals.E2E_USER_A_ID ?? '',
      userBId: idVals.E2E_USER_B_ID ?? '',
    },
    midscene: {
      apiKey: midsceneApiKey ?? '',
      modelName: get('MIDSCENE_MODEL_NAME') ?? 'gpt-4o',
      baseUrl: midsceneBaseUrl,
      cache: midsceneCache,
    },
    ciReady,
    // T-0000P 扩展：Azure / 自定义 baseURL 可选字段（不冻结到主对象，由 writeProcessEnv 透传）
    _azureEndpoint: azureEndpoint,
    _azureApiKey: azureApiKey,
  };
  // 深冻结 nested 对象
  Object.freeze(env.tokens);
  Object.freeze(env.ids);
  Object.freeze(env.midscene);
  return Object.freeze(env);
}

/** 把 E2EEnv 字段写回 process.env，供 worker / spawned 子进程消费。 */
export function writeProcessEnv(env: E2EEnv): void {
  process.env.E2E_PROFILE = env.profile;
  process.env.E2E_ALLOW_WRITES = env.allowWrites ? '1' : '0';
  process.env.APP_SERVER_BASE_URL = env.appServerBaseUrl;
  process.env.ADMIN_SERVER_BASE_URL = env.adminServerBaseUrl;
  process.env.ADMIN_WEB_URL = env.adminWebUrl;
  // T-0000J §2.3：双 key fallback。playwright.config.ts use.baseURL 优先读 runtime 私有 key，
  // 退回 shell 预 export 的 ADMIN_WEB_URL；避免 globalSetup 求值时序与 shell env 双向冲突。
  process.env._E2E_RUNTIME_ADMIN_WEB_URL = env.adminWebUrl;
  process.env.APP_WS_URL = env.appWsUrl;
  if (env.databaseUrl) process.env.DATABASE_URL = env.databaseUrl;
  if (env.redisUrl) process.env.REDIS_URL = env.redisUrl;
  if (env.androidAppId) process.env.ANDROID_APP_ID = env.androidAppId;

  process.env.E2E_VALID_TOKEN = env.tokens.valid;
  process.env.E2E_EXPIRED_TOKEN = env.tokens.expired;
  process.env.E2E_ADMIN_TOKEN = env.tokens.admin;
  process.env.E2E_OP_TOKEN = env.tokens.op;
  process.env.E2E_CS_TOKEN = env.tokens.cs;
  process.env.E2E_FIN_TOKEN = env.tokens.fin;
  process.env.E2E_EXPIRED_ADMIN_TOKEN = env.tokens.expiredAdmin;

  process.env.E2E_ROOM_ID = env.ids.roomId;
  process.env.E2E_USER_A_ID = env.ids.userAId;
  process.env.E2E_USER_B_ID = env.ids.userBId;

  // T-0000P §二 改造点位 1：Midscene env 双注入（兼容性保障）
  if (env.midscene.apiKey) {
    process.env.MIDSCENE_MODEL_API_KEY = env.midscene.apiKey;
    process.env.OPENAI_API_KEY = env.midscene.apiKey;
  }
  process.env.MIDSCENE_MODEL_NAME = env.midscene.modelName;
  if (env.midscene.baseUrl) {
    process.env.MIDSCENE_MODEL_BASE_URL = env.midscene.baseUrl;
    process.env.MIDSCENE_OPENAI_BASE_URL = env.midscene.baseUrl; // 旧版兼容
  }
  process.env.MIDSCENE_CACHE = env.midscene.cache ? '1' : '0';
  // Azure 可选字段透传
  if (env._azureEndpoint) process.env.AZURE_OPENAI_ENDPOINT = env._azureEndpoint;
  if (env._azureApiKey) process.env.AZURE_OPENAI_API_KEY = env._azureApiKey;

  process.env.CI_E2E_READY = env.ciReady ? '1' : '0';

  // 缺陷 2 修复（batch-e2e-foundation-01 第 1 轮）：
  //   桥接根 .env.{profile} → Android BuildConfig 注入路径。
  //   gradle 侧 `resolveConfigValue(localProperties, "voiceRoomApiBaseUrl",
  //   "VOICE_ROOM_API_BASE_URL", default)` 会从 process.env 读取这些键，
  //   实现「根 .env → envLoader → process.env → gradlew → BuildConfig」单一事实源链路。
  //   见 doc/tests/E2E_RUNBOOK.md「Android E2E 注入路径」段。
  process.env.VOICE_ROOM_API_BASE_URL = env.appServerBaseUrl;
  process.env.VOICE_ROOM_WS_URL = env.appWsUrl;
  // analyticsEndpoint 不在 E2EEnv 主字段中（24 字段表外），
  // 留空允许 gradle fallback 到 `${API_BASE_URL}/v1/events/batch`。
  process.env.VOICE_ROOM_ANALYTICS_ENDPOINT = process.env.VOICE_ROOM_ANALYTICS_ENDPOINT ?? '';
  // local flavor 默认（与 build.gradle.kts 默认值保持一致），allows env 覆盖
  process.env.VOICE_ROOM_ENVIRONMENT = process.env.VOICE_ROOM_ENVIRONMENT ?? env.profile;
}

/**
 * T-0000K §2.7 安全红线 #2：持久化 .e2e-runtime.json 前的脱敏副本。
 *
 * 黑名单：MIDSCENE_MODEL_API_KEY 永不写入磁盘运行时档案。
 * 副作用：返回的副本 `midscene.apiKey === ''`；其他字段原样保留。
 *
 * Worker 端读取 runtime json 后，仍需经 process.env（writeProcessEnv 已注入）取真实 Key。
 */
export function sanitizeEnvForRuntimeJson(env: E2EEnv): E2EEnv {
  const safe: E2EEnv = {
    ...env,
    tokens: { ...env.tokens },
    ids: { ...env.ids },
    midscene: {
      ...env.midscene,
      apiKey: '',
    },
    // T-0000P：清除 Azure 敏感字段
    _azureEndpoint: undefined,
    _azureApiKey: undefined,
  };
  return safe;
}
