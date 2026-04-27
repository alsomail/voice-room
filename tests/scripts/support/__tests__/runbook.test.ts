/**
 * T-0000L: E2E_RUNBOOK.md 静态体检（U-1 ~ U-11）
 *
 * 运行：npx playwright test --config=playwright.unit.config.ts \
 *         tests/scripts/support/__tests__/runbook.test.ts
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

const REPO_ROOT = path.resolve(__dirname, '../../../..');
const RUNBOOK = path.join(REPO_ROOT, 'doc/tests/E2E_RUNBOOK.md');
const INDEX = path.join(REPO_ROOT, 'doc/tests/index.md');
const PKG = path.join(REPO_ROOT, 'package.json');

function readRunbook(): string {
  return fs.readFileSync(RUNBOOK, 'utf8');
}

test('U-1 RUNBOOK-EXISTS：文件存在且 ≥ 2KB / ≥ 100 行', () => {
  expect(fs.existsSync(RUNBOOK)).toBe(true);
  const stat = fs.statSync(RUNBOOK);
  expect(stat.size).toBeGreaterThan(2000);
  const lines = readRunbook().split('\n').length;
  expect(lines).toBeGreaterThanOrEqual(100);
});

test('U-2 COLD-START-5-STEPS：含「冷启动」章节 + 1.~5. 编号 step', () => {
  const text = readRunbook();
  expect(/冷启动/.test(text)).toBe(true);
  // 5 个连续编号步骤（行首 1. 2. 3. 4. 5.）
  for (const n of [1, 2, 3, 4, 5]) {
    const re = new RegExp(`^\\s{0,3}${n}\\.\\s`, 'm');
    expect(re.test(text), `缺少编号步骤 ${n}.`).toBe(true);
  }
});

test('U-3 PREFLIGHT-5-PORTS：覆盖 ≥ 5 个唯一端口字面', () => {
  const text = readRunbook();
  const candidates = ['3000', '3001', '5173', '5432', '6379', '8080', '8081'];
  const hit = new Set<string>();
  for (const p of candidates) {
    const re = new RegExp(`\\b${p}\\b`);
    if (re.test(text)) hit.add(p);
  }
  expect(hit.size).toBeGreaterThanOrEqual(5);
  // 必含 5173 / 5432 / 6379（docker-compose 固化）
  expect(hit.has('5173')).toBe(true);
  expect(hit.has('5432')).toBe(true);
  expect(hit.has('6379')).toBe(true);
});

test('U-4 CMD-MATRIX-NO-HALLUCINATION：所有 npm run e2e:* / db:* / preflight 名 ⊆ package.json scripts', () => {
  const pkg = JSON.parse(fs.readFileSync(PKG, 'utf8'));
  const scriptNames: string[] = Object.keys(pkg.scripts || {});
  const scriptSet = new Set(scriptNames);
  const text = readRunbook();
  // 解析所有 `npm run <name>` 出现（排除删除线 ~~...~~ 包裹的）
  const matches = [...text.matchAll(/npm run ([a-z][a-z0-9:-]*)/g)];
  const referenced = new Set<string>();
  for (const m of matches) {
    const name = m[1];
    // 找该匹配在文本中位置，检查是否在 ~~...~~ 删除线内（同一行内）
    const idx = m.index ?? 0;
    // 同一行内向前找最近 ~~，向后找最近 ~~
    const lineStart = text.lastIndexOf('\n', idx) + 1;
    const lineEnd = text.indexOf('\n', idx);
    const line = text.slice(lineStart, lineEnd === -1 ? text.length : lineEnd);
    // 行内若整段被 ~~ ~~ 包裹（命令名出现在删除线对内），跳过
    const before = line.slice(0, idx - lineStart);
    const tildeBefore = (before.match(/~~/g) || []).length;
    if (tildeBefore % 2 === 1) continue; // 在 ~~ ... 之内
    referenced.add(name);
  }
  // 所有引用必须真实存在
  const phantom = [...referenced].filter(n => !scriptSet.has(n));
  expect(phantom, `RUNBOOK 引用的不存在脚本：${phantom.join(', ')}`).toEqual([]);
  // 至少覆盖 4 个核心脚本（防写得太少）
  for (const must of ['preflight', 'e2e:local', 'e2e:prod-smoke']) {
    expect(referenced.has(must), `命令矩阵缺少 ${must}`).toBe(true);
  }
});

test('U-5 MIDSCENE-LINK：含 MIDSCENE_SETUP 引用', () => {
  const text = readRunbook();
  expect(/MIDSCENE_SETUP/.test(text)).toBe(true);
  // 首选相对路径形式
  expect(text.includes('./MIDSCENE_SETUP.md')).toBe(true);
});

test('U-6 BASEURL-FALLBACK：含 _E2E_RUNTIME_ADMIN_WEB_URL 或 baseURL 双 key fallback 关键字', () => {
  const text = readRunbook();
  const ok =
    text.includes('_E2E_RUNTIME_ADMIN_WEB_URL') ||
    /baseURL.*双\s*key/.test(text) ||
    /T-0000J/.test(text);
  expect(ok).toBe(true);
});

test('U-7 WIN-DOUBLE-QUOTE：含 staging/prod-safe Windows 双引号警示 + 字面 --grep "@prod-safe"', () => {
  const text = readRunbook();
  expect(text.includes('--grep "@prod-safe"')).toBe(true);
  // 提及 Windows / PowerShell / 双引号 之一
  expect(/(Windows|PowerShell|双引号)/.test(text)).toBe(true);
});

test('U-8 INDEX-LINKED：doc/tests/index.md 已正式链接 RUNBOOK 且非占位', () => {
  const idx = fs.readFileSync(INDEX, 'utf8');
  expect(idx.includes('E2E_RUNBOOK.md')).toBe(true);
  // 在含 E2E_RUNBOOK 的行不得出现「待编写」「待编 / TODO / 占位」字样
  const lines = idx.split('\n').filter(l => l.includes('E2E_RUNBOOK'));
  expect(lines.length).toBeGreaterThan(0);
  for (const line of lines) {
    expect(/待编写|TODO|占位/.test(line), `index.md 中 RUNBOOK 链接行仍含占位: ${line}`).toBe(false);
  }
});

test('U-9 TIME-BUDGET-5MIN：含「5 分钟 / 5min / ≤5min」时长目标', () => {
  const text = readRunbook();
  const ok = /5\s*分钟/.test(text) || /≤\s*5\s*min/i.test(text) || /\b5\s*min(ute)?s?\b/i.test(text);
  expect(ok).toBe(true);
});

test('U-10 NO-PLAINTEXT-CRED：反向断言无明文凭据', () => {
  const text = readRunbook();
  // sk-XXXX 形态 API Key
  expect(/sk-[A-Za-z0-9]{20,}/.test(text)).toBe(false);
  // password = 明文（排除占位 <...> 与 ***）
  const pwdMatches = [...text.matchAll(/password\s*[:=]\s*([^\s<*`'"]+)/gi)];
  for (const m of pwdMatches) {
    const v = m[1];
    expect(v.startsWith('<') || /^\*+$/.test(v), `疑似明文 password 值: ${v}`).toBe(true);
  }
});

test('U-11 FAQ-COUNT：FAQ 章节 ≥ 6 条编号', () => {
  const text = readRunbook();
  // 取「FAQ」章节起到下一二级标题止
  const m = text.match(/(##\s*§?7[^\n]*FAQ[\s\S]*?)(?=\n##\s|$)/);
  expect(m, '未找到 FAQ 章节').toBeTruthy();
  const section = m![1];
  // 计数行首编号 1. 2. 3. ... 或 ### Q1 ~ Q6
  const numbered = (section.match(/^\s{0,3}\d+\.\s/gm) || []).length;
  const qHeads = (section.match(/^#{2,4}\s*Q\d+/gm) || []).length;
  expect(Math.max(numbered, qHeads)).toBeGreaterThanOrEqual(6);
});

// ─────────────────────────────────────────────────────────────────────────────
// 缺陷 4/5/6 修复（batch-e2e-foundation-01 第 1 轮）静态守护
// ─────────────────────────────────────────────────────────────────────────────

test('U-12 BUDGET-INCLUDES-CARGO-BUILD：5min 预算说明必须含 cargo / 首次冷启动 8~18min 或预热段', () => {
  const text = readRunbook();
  const ok =
    /cargo\s*build/i.test(text) ||
    /首次\s*\d+\s*~\s*\d+\s*min/.test(text) ||
    /冷启动.*cargo/i.test(text);
  expect(ok, '缺陷 4：RUNBOOK 5min 预算未把 cargo 冷编译纳入说明').toBe(true);
});

test('U-13 STEP5-USES-LOCAL-PROFILE：Step 5 推荐命令必须含 e2e:local 而非以 prod-smoke 作为冷启动首条命令', () => {
  const text = readRunbook();
  // 抓取「冷启动 5 步」段落到下一个 ## 章节
  const m = text.match(/##\s*§?2[^\n]*冷启动[\s\S]*?(?=\n##\s)/);
  expect(m, '未找到冷启动 5 步章节').toBeTruthy();
  const section = m![0];
  expect(section, '冷启动 Step 5 必须示范 e2e:local --list 或 --grep').toMatch(
    /npm run e2e:local\s+--\s+(--list|--grep)/,
  );
});

test('U-14 DOCKER-COMPOSE-DESIGN-NOTE：必须明示 docker-compose 仅托管 PG/Redis 的设计取舍', () => {
  const text = readRunbook();
  expect(/docker-compose|docker compose/.test(text)).toBe(true);
  // 必须含「仅托管」或「PG/Redis」+「业务服务」字样
  const ok = /仅托管.*Postgres|仅托管.*PG|cargo.*本地起|业务服务.*本地/.test(text);
  expect(ok, '缺陷 6：RUNBOOK §2 缺少 docker-compose 设计取舍说明').toBe(true);
});

test('U-15 E2E-UP-DOC：RUNBOOK 必须引用 npm run e2e:up 一键命令', () => {
  const text = readRunbook();
  expect(text).toContain('npm run e2e:up');
});

test('U-16 ANDROID-INJECT-PATH：必须含 Android E2E 注入路径段（缺陷 2）', () => {
  const text = readRunbook();
  const ok =
    /Android.*E2E.*注入/i.test(text) ||
    /VOICE_ROOM_API_BASE_URL/.test(text);
  expect(ok, '缺陷 2：RUNBOOK 缺少 Android 注入路径说明').toBe(true);
});
