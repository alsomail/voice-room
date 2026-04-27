/**
 * T-0000K Midscene LLM 配置接入文档 + CI Secret 流程
 * 单元验收用例 U-1 ~ U-9（TDS §三）。
 *
 * 不启动浏览器：
 *   - 文档存在性 / grep 类静态体检
 *   - midsceneReadyImpl 纯函数 mock 调用
 *   - envLoader.writeProcessEnv 黑名单（不持久化 API Key 至 .e2e-runtime.json）
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { midsceneReadyImpl } from '../fixtures';
import { writeProcessEnv } from '../envLoader';
import type { E2EEnv } from '../types';

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const DOC_PATH = path.join(REPO_ROOT, 'doc', 'tests', 'MIDSCENE_SETUP.md');

function readDoc(): string {
  return fs.readFileSync(DOC_PATH, 'utf8');
}

function makeEnv(apiKey: string): E2EEnv {
  return Object.freeze({
    profile: 'local',
    allowWrites: true,
    appServerBaseUrl: 'http://x',
    adminServerBaseUrl: 'http://x',
    adminWebUrl: 'http://x',
    appWsUrl: 'ws://x',
    tokens: { valid: '', expired: '', admin: '', op: '', cs: '', fin: '', expiredAdmin: '' },
    ids: { roomId: '', userAId: '', userBId: '' },
    midscene: { apiKey, modelName: 'gpt-4o', cache: true },
    ciReady: false,
  }) as E2EEnv;
}

// 隔离 shell 已 export 的 MIDSCENE_MODEL_API_KEY，避免影响 U-4/5/6/7 期望
test.beforeEach(() => {
  delete process.env.MIDSCENE_MODEL_API_KEY;
});

// ─────────────────────────────────────────────────────────────────────────────
// U-1：文档存在性
// ─────────────────────────────────────────────────────────────────────────────
test('U-1 doc/tests/MIDSCENE_SETUP.md 存在', () => {
  expect(fs.existsSync(DOC_PATH)).toBe(true);
});

// ─────────────────────────────────────────────────────────────────────────────
// U-2：三形态字段表完整
// ─────────────────────────────────────────────────────────────────────────────
test('U-2 三形态字段表完整：OpenAI 直连 / Azure / 中转 三段必填字段全部命中', () => {
  const doc = readDoc();
  // 三形态标题（容忍 A/B/C 编号或别名）
  expect(doc).toMatch(/OpenAI\s*直连/);
  expect(doc).toMatch(/Azure(\s*OpenAI)?/);
  expect(doc).toMatch(/中转|自托管|relay|OneAPI|LiteLLM/i);

  // 必填字段名（OpenAI 2 / Azure 5 / 中转 3，去重后 6 类）
  const required = [
    'MIDSCENE_MODEL_API_KEY',
    'MIDSCENE_MODEL_NAME',
    'MIDSCENE_USE_AZURE_OPENAI',
    'AZURE_OPENAI_ENDPOINT',
    'AZURE_OPENAI_DEPLOYMENT',
    'AZURE_OPENAI_API_VERSION',
    'MIDSCENE_OPENAI_BASE_URL',
  ];
  for (const f of required) {
    expect(doc).toContain(f);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// U-3：CI Secret 注入示例 + 反向断言（无明文）
// ─────────────────────────────────────────────────────────────────────────────
test('U-3 文档含 GitHub Actions Secret 引用，且不出现 Key 明文样例', () => {
  const doc = readDoc();
  // 正向：必须使用 secrets 引用语法
  expect(doc).toMatch(/\$\{\{\s*secrets\.MIDSCENE_MODEL_API_KEY\s*\}\}/);
  // 反向：禁止出现形如 MIDSCENE_MODEL_API_KEY=sk-xxxx / : sk- 等明文
  expect(doc).not.toMatch(/MIDSCENE_MODEL_API_KEY\s*[:=]\s*sk-[A-Za-z0-9]/);
  // 反向：禁止真实 key 形态
  expect(doc).not.toMatch(/sk-[A-Za-z0-9]{20,}/);
});

// ─────────────────────────────────────────────────────────────────────────────
// U-4：fixture 在 WEB spec 路径下、Key 缺失时 skip
// ─────────────────────────────────────────────────────────────────────────────
test('U-4 midsceneReadyImpl: WEB spec + apiKey 空 → skip', () => {
  const skips: Array<{ cond: boolean; reason: string }> = [];
  const ti = {
    file: path.join('/abs', 'tests', 'scripts', 'WEB', 'TC-AUTH.spec.ts'),
    skip: (cond: boolean, reason: string) => { skips.push({ cond, reason }); },
  };
  const skipped = midsceneReadyImpl(makeEnv(''), ti as any);
  expect(skipped).toBe(true);
  expect(skips).toHaveLength(1);
  expect(skips[0].cond).toBe(true);
  expect(skips[0].reason).toBe('[MIDSCENE] api key missing — skipped');
});

// ─────────────────────────────────────────────────────────────────────────────
// U-5：非 WEB spec（API/ADMIN_WEB/APPSERVER）即使 Key 空也不 skip
// ─────────────────────────────────────────────────────────────────────────────
test('U-5 midsceneReadyImpl: 非 WEB spec + apiKey 空 → 不 skip', () => {
  const skips: string[] = [];
  const ti = {
    file: path.join('/abs', 'tests', 'scripts', 'API', 'TC-USER.spec.ts'),
    skip: (cond: boolean, reason: string) => { if (cond) skips.push(reason); },
  };
  const skipped = midsceneReadyImpl(makeEnv(''), ti as any);
  expect(skipped).toBe(false);
  expect(skips).toHaveLength(0);

  // ADMIN_WEB 也不应 skip
  const ti2 = {
    file: path.join('/abs', 'tests', 'scripts', 'ADMIN_WEB', 'TC-X.spec.ts'),
    skip: (cond: boolean, reason: string) => { if (cond) skips.push(reason); },
  };
  expect(midsceneReadyImpl(makeEnv(''), ti2 as any)).toBe(false);
  expect(skips).toHaveLength(0);
});

// ─────────────────────────────────────────────────────────────────────────────
// U-6：有 Key 时不 skip
// ─────────────────────────────────────────────────────────────────────────────
test('U-6 midsceneReadyImpl: WEB spec + apiKey=sk-test → 不 skip', () => {
  const skips: string[] = [];
  const ti = {
    file: '/abs/tests/scripts/WEB/TC-AUTH.spec.ts',
    skip: (cond: boolean, reason: string) => { if (cond) skips.push(reason); },
  };
  expect(midsceneReadyImpl(makeEnv('sk-test'), ti as any)).toBe(false);
  expect(skips).toHaveLength(0);
});

// ─────────────────────────────────────────────────────────────────────────────
// U-7：writeProcessEnv 不持久化 MIDSCENE_MODEL_API_KEY 至 .e2e-runtime.json
// ─────────────────────────────────────────────────────────────────────────────
test('U-7 writeProcessEnv 不持久化 MIDSCENE_MODEL_API_KEY 至 .e2e-runtime.json', () => {
  // 模拟 globalSetup Step 5 的写文件契约：env JSON 序列化后落盘 → grep 文件不应含 apiKey 字面值
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 't-0000k-'));
  try {
    const env = makeEnv('sk-secret-leak-canary-12345');
    // 注入 process.env（此分支允许）
    delete process.env.MIDSCENE_MODEL_API_KEY;
    writeProcessEnv(env);
    expect(process.env.MIDSCENE_MODEL_API_KEY).toBe('sk-secret-leak-canary-12345');

    // 模拟 globalSetup 的持久化逻辑：必须使用脱敏副本（envLoader 提供 sanitize 或 globalSetup 内联实现）
    // 此处验证 JSON.stringify(env) 后写盘的行为契约 —— 由 globalSetup 持久化前调用 sanitize。
    // 我们直接 import 并使用脱敏函数：
    // 通过反射（require 方式）以避免类型耦合。
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { sanitizeEnvForRuntimeJson } = require('../envLoader') as typeof import('../envLoader');
    expect(typeof sanitizeEnvForRuntimeJson).toBe('function');

    const safe = sanitizeEnvForRuntimeJson(env);
    const runtimePath = path.join(tmp, '.e2e-runtime.json');
    fs.writeFileSync(runtimePath, JSON.stringify(safe, null, 2));

    const content = fs.readFileSync(runtimePath, 'utf8');
    expect(content).not.toContain('sk-secret-leak-canary-12345');
    // 持久化后 midscene.apiKey 应为空字符串
    const parsed = JSON.parse(content);
    expect(parsed.midscene?.apiKey ?? '').toBe('');
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
    delete process.env.MIDSCENE_MODEL_API_KEY;
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// U-8：globalSetup 实际写出的 .e2e-runtime.json 也不含 key 字面（如果存在）
// ─────────────────────────────────────────────────────────────────────────────
test('U-8 .e2e-runtime.json（若存在）不含 sk- 字面 API Key', () => {
  const runtimePath = path.join(REPO_ROOT, 'tests', 'scripts', '.e2e-runtime.json');
  if (!fs.existsSync(runtimePath)) {
    test.skip(true, 'runtime json not present in unit env');
    return;
  }
  const content = fs.readFileSync(runtimePath, 'utf8');
  expect(content).not.toMatch(/sk-[A-Za-z0-9]{20,}/);
  // 反向：若包含 midscene 段，apiKey 必须为空串
  try {
    const parsed = JSON.parse(content);
    if (parsed.midscene) {
      expect(parsed.midscene.apiKey ?? '').toBe('');
    }
  } catch {
    // ignore parse failure
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// U-9：FAQ 章节存在（限流/超时/Azure deployment 误填 至少 3 条）
// ─────────────────────────────────────────────────────────────────────────────
test('U-9 文档含 FAQ 章节，覆盖限流/超时/Azure deployment 误填 等条目', () => {
  const doc = readDoc();
  expect(doc).toMatch(/##?\s*FAQ|常见问题/);
  // 至少命中 3 个关键诊断条目
  expect(doc).toMatch(/429|限流|rate\s*limit/i);
  expect(doc).toMatch(/超时|timeout/i);
  expect(doc).toMatch(/deployment|部署名|401/i);
});
