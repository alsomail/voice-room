/**
 * T-0000J: E2E 用例 baseURL 修复 + 密码 typo 清理 + @prod-safe 标签 TDD 验收
 *
 * 覆盖 §三 U-1 ~ U-12（按 TDS 编号注释）：
 *   - U-1  shell grep typo 零容忍
 *   - U-2  shell grep localhost 端口零容忍
 *   - U-3  shell grep dotenv 顶层 import 零容忍
 *   - U-4  playwright.config.ts dotenv import 零容忍
 *   - U-5  playwright.config.ts use.baseURL 双 key fallback
 *   - U-6  page.goto 相对路径
 *   - U-7  @prod-safe 用例 read-only 静态体检（grep INSERT/UPDATE/.post/.put 等 0 命中）
 *   - U-8  playwright list --grep "@prod-safe" 命中 ≥ 5（按 spec+title 去重）
 *   - U-9  envLoader writeProcessEnv → playwright.config use.baseURL 端到端
 *   - U-10 fs.readFileSync 全量扫描 typo 零容忍（与 U-1 双源）
 *   - U-11 fuzzy 标签拼写零容忍
 *   - U-12 globalSetup 注入 ADMIN_WEB_URL + _E2E_RUNTIME_ADMIN_WEB_URL 双 key
 *
 * Runner: playwright.unit.config.ts（不启 globalSetup）。
 */
import { test, expect } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { writeProcessEnv } from '../envLoader';

const REPO_ROOT = path.resolve(__dirname, '../../../..');
const TESTS_DIR = path.join(REPO_ROOT, 'tests');
const SCRIPTS_DIR = path.join(REPO_ROOT, 'tests/scripts');
const PW_CONFIG = path.join(REPO_ROOT, 'playwright.config.ts');

/** 递归收集 .spec.ts 路径（排除 support/__tests__）。 */
function listSpecFiles(roots: string[]): string[] {
  const out: string[] = [];
  for (const root of roots) {
    if (!fs.existsSync(root)) continue;
    const entries = fs.readdirSync(root, { withFileTypes: true });
    for (const e of entries) {
      const full = path.join(root, e.name);
      if (e.isDirectory()) {
        if (full.includes('__tests__')) continue;
        out.push(...listSpecFiles([full]));
      } else if (e.isFile() && e.name.endsWith('.spec.ts')) {
        out.push(full);
      }
    }
  }
  return out;
}

function shellGrep(pattern: string, paths: string[], extraArgs: string[] = []): { code: number; stdout: string } {
  const r = spawnSync('grep', [...extraArgs, pattern, ...paths], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
  });
  return { code: r.status ?? -1, stdout: r.stdout ?? '' };
}

// ─────────────────────── U-1 / U-10 typo 零容忍（双源） ───────────────────────
test.describe('T-0000J U-1 / U-10：DB 密码 typo `app_server_pwd` 双源零容忍', () => {
  test('U-1: shell grep tests/ playwright.config.ts 退出码 1（0 命中）', () => {
    const r = shellGrep('app_server_pwd', ['tests/', 'playwright.config.ts'], [
      '-rn',
      '--exclude-dir=__tests__',
    ]);
    // grep 0 命中 → exit 1；任意命中 → exit 0
    expect(r.code, `不应命中：\n${r.stdout}`).toBe(1);
    expect(r.stdout.trim()).toBe('');
  });

  test('U-10: fs.readFileSync 全量扫描 tests/ 0 命中（与 U-1 双源）', () => {
    const specs = listSpecFiles([TESTS_DIR]);
    const hits: string[] = [];
    for (const f of specs) {
      // 自身（U-1/U-10 测试文件）含 typo 字符串作为搜索 needle，需排除
      if (f.includes(`${path.sep}__tests__${path.sep}`)) continue;
      const text = fs.readFileSync(f, 'utf8');
      if (text.includes('app_server_pwd')) hits.push(f);
    }
    expect(hits, `命中文件：${hits.join(', ')}`).toEqual([]);
  });
});

// ─────────────────────── U-2 localhost 端口零容忍 ───────────────────────
test.describe('T-0000J U-2：硬编码 localhost:3000/3001/5173 零容忍', () => {
  test('U-2: grep -rnE "localhost:(3000|3001|5173)" tests/scripts/{API,E2E,WEB} 0 命中', () => {
    const r = shellGrep('localhost:(3000|3001|5173)', [
      'tests/scripts/API/',
      'tests/scripts/E2E/',
      'tests/scripts/WEB/',
    ], ['-rnE']);
    expect(r.code, `不应命中：\n${r.stdout}`).toBe(1);
    expect(r.stdout.trim()).toBe('');
  });
});

// ─────────────────────── U-3 dotenv 顶层 import 零容忍 ───────────────────────
test.describe('T-0000J U-3：spec 顶层 dotenv/config import 零容忍', () => {
  test("U-3: grep \"import 'dotenv/config'\" tests/scripts/ 0 命中", () => {
    const r = shellGrep("import 'dotenv/config'", ['tests/scripts/'], [
      '-rn',
      '--exclude-dir=__tests__',
    ]);
    expect(r.code, `不应命中：\n${r.stdout}`).toBe(1);
    expect(r.stdout.trim()).toBe('');
  });
});

// ─────────────────────── U-4 playwright.config dotenv import 零容忍 ───────────
test.describe('T-0000J U-4：playwright.config.ts dotenv import 零容忍', () => {
  test("U-4: 不出现 import 'dotenv/config' / dotenv.config()", () => {
    const txt = fs.readFileSync(PW_CONFIG, 'utf8');
    expect(txt).not.toMatch(/import\s+['"]dotenv\/config['"]/);
    expect(txt).not.toMatch(/dotenv\.config\s*\(/);
  });
});

// ─────────────────────── U-5 use.baseURL 双 key fallback ───────────────────────
test.describe('T-0000J U-5：playwright.config.ts use.baseURL 双 key fallback', () => {
  test('U-5: baseURL 表达式同时含 _E2E_RUNTIME_ADMIN_WEB_URL 与 ADMIN_WEB_URL，且无 localhost 字面量', () => {
    const txt = fs.readFileSync(PW_CONFIG, 'utf8');
    // 找 baseURL 行
    const m = txt.match(/baseURL\s*:\s*([^,\n]+)/);
    expect(m, 'baseURL 行未找到').not.toBeNull();
    const expr = m![1];
    expect(expr, `baseURL 表达式: ${expr}`).toContain('_E2E_RUNTIME_ADMIN_WEB_URL');
    expect(expr).toContain('ADMIN_WEB_URL');
    expect(txt).not.toContain("'http://localhost:5173'");
    expect(txt).not.toContain('"http://localhost:5173"');
  });
});

// ─────────────────────── U-6 page.goto 相对路径 ───────────────────────
test.describe('T-0000J U-6：WEB 用例 page.goto 相对路径', () => {
  test('U-6: 不出现 page.goto(`${ADMIN_WEB_URL}…）；至少 1 处 page.goto(\'/…\')', () => {
    const r1 = shellGrep('page.goto(`${ADMIN_WEB_URL}', ['tests/scripts/WEB/'], ['-rnF']);
    expect(r1.code, `不应模板字符串硬拼：\n${r1.stdout}`).toBe(1);

    const r2 = shellGrep("page.goto('/", ['tests/scripts/WEB/'], ['-rn']);
    expect(r2.code, '应至少 1 处相对路径').toBe(0);
    expect(r2.stdout.split('\n').filter(Boolean).length).toBeGreaterThanOrEqual(1);
  });
});

// ─────────────────────── U-7 @prod-safe read-only 静态体检 ───────────────────────
type ProdSafeTest = { file: string; titleLine: number; body: string };

/**
 * 解析 spec 文件，抽取打了 @prod-safe 标签（title 含或 metadata `tag: '@prod-safe'`）的 test 块 body。
 * 实现采用「行扫描 + 大括号配对」的简化解析，足以应付当前 spec 风格。
 */
function extractProdSafeBodies(file: string): ProdSafeTest[] {
  const text = fs.readFileSync(file, 'utf8');
  const lines = text.split('\n');
  const out: ProdSafeTest[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // 命中 test( 起始行；同一文件内只对单条 test 处理（不递归 describe.tag，因为 metadata 在 test() 第二参数最直接）
    const isTestStart = /^\s*test\s*\(/.test(line) || /^\s*test\.only\s*\(/.test(line);
    if (!isTestStart) continue;
    // 收集到最近的 } 闭合（用括号配对到对应 ');' 行）
    let depthBrace = 0;
    let depthParen = 0;
    let started = false;
    let buf = '';
    let j = i;
    for (; j < lines.length; j++) {
      const l = lines[j];
      buf += l + '\n';
      for (const ch of l) {
        if (ch === '(') { depthParen++; started = true; }
        else if (ch === ')') depthParen--;
        else if (ch === '{') depthBrace++;
        else if (ch === '}') depthBrace--;
      }
      if (started && depthParen === 0 && depthBrace === 0) break;
    }
    // 判断这块是否打了 @prod-safe（title 含 / metadata tag 含）
    if (/@prod-safe\b/.test(buf) || /tag\s*:\s*['"]@prod-safe['"]/.test(buf) ||
        /tag\s*:\s*\[\s*['"]@prod-safe['"]/.test(buf)) {
      out.push({ file, titleLine: i + 1, body: buf });
    }
    i = j;
  }
  // 同时考虑 describe 块整体打标场景：若整个 describe 块 title 含 @prod-safe，则其内的所有 test 块视为命中
  // 简化：扫描 describe 起始行，若标题有 @prod-safe，把该块内每条 test body 加入
  const reDesc = /^\s*test\.describe\s*\(\s*['"`]([^'"`]*@prod-safe[^'"`]*)['"`]/;
  for (let i = 0; i < lines.length; i++) {
    if (!reDesc.test(lines[i])) continue;
    let depthBrace = 0, depthParen = 0, started = false;
    let buf = '';
    let j = i;
    for (; j < lines.length; j++) {
      buf += lines[j] + '\n';
      for (const ch of lines[j]) {
        if (ch === '(') { depthParen++; started = true; }
        else if (ch === ')') depthParen--;
        else if (ch === '{') depthBrace++;
        else if (ch === '}') depthBrace--;
      }
      if (started && depthParen === 0 && depthBrace === 0) break;
    }
    out.push({ file, titleLine: i + 1, body: buf });
    i = j;
  }
  return out;
}

test.describe('T-0000J U-7：@prod-safe 用例 read-only 静态体检', () => {
  test('U-7: 打标用例源码不含 INSERT/UPDATE/DELETE/TRUNCATE/.post/.put/.patch/.delete', () => {
    const specs = listSpecFiles([SCRIPTS_DIR]);
    const tagged: ProdSafeTest[] = [];
    for (const f of specs) tagged.push(...extractProdSafeBodies(f));
    expect(tagged.length, '应至少有 1 条 @prod-safe 用例').toBeGreaterThanOrEqual(1);

    const dangerRe = /\b(INSERT|UPDATE|DELETE|TRUNCATE)\b|\.(post|put|patch|delete)\s*\(/;
    const violators: string[] = [];
    for (const t of tagged) {
      if (dangerRe.test(t.body)) {
        violators.push(`${path.relative(REPO_ROOT, t.file)}:${t.titleLine}`);
      }
    }
    expect(violators, `read-only 违规命中：\n${violators.join('\n')}`).toEqual([]);
  });
});

// ─────────────────────── U-8 @prod-safe 数量 ≥ 5 ───────────────────────
test.describe('T-0000J U-8：@prod-safe 标签命中数 ≥ 5（按 spec+title 去重）', () => {
  test('U-8: 静态扫描所有 spec.ts，去重后 ≥ 5', () => {
    const specs = listSpecFiles([SCRIPTS_DIR]);
    const set = new Set<string>();
    for (const f of specs) {
      const blocks = extractProdSafeBodies(f);
      for (const b of blocks) {
        // 用 file + titleLine 去重（每条 test 唯一）
        set.add(`${path.relative(REPO_ROOT, f)}:${b.titleLine}`);
      }
    }
    expect(set.size, `命中清单：\n${[...set].join('\n')}`).toBeGreaterThanOrEqual(5);
  });
});

// ─────────────────────── U-9 envLoader 注入 baseURL 端到端 ───────────────────────
test.describe('T-0000J U-9：envLoader writeProcessEnv → playwright.config use.baseURL 端到端', () => {
  test('U-9: writeProcessEnv 双写 ADMIN_WEB_URL + _E2E_RUNTIME_ADMIN_WEB_URL；defineConfig.use.baseURL 等于注入值', async () => {
    // 备份 env
    const backup: Record<string, string | undefined> = {};
    const keys = ['ADMIN_WEB_URL', '_E2E_RUNTIME_ADMIN_WEB_URL'];
    for (const k of keys) backup[k] = process.env[k];

    try {
      // 清理
      for (const k of keys) delete process.env[k];

      // 构造一个 minimal E2EEnv（仅消费 adminWebUrl 字段）
      const env: any = {
        profile: 'local',
        allowWrites: true,
        appServerBaseUrl: 'http://app:3000',
        adminServerBaseUrl: 'http://admin:3001',
        adminWebUrl: 'http://admin-web.test:5173',
        appWsUrl: 'ws://app:3000/ws',
        databaseUrl: 'postgres://x',
        redisUrl: 'redis://x',
        androidAppId: 'com.x',
        tokens: { valid: 'v', expired: 'e', admin: 'a', op: 'o', cs: 'c', fin: 'f', expiredAdmin: 'ea' },
        ids: { roomId: 'r', userAId: 'a', userBId: 'b' },
        midscene: { apiKey: '', modelName: 'gpt', baseUrl: '', cache: false },
        ciReady: false,
      };
      writeProcessEnv(env);

      expect(process.env.ADMIN_WEB_URL).toBe('http://admin-web.test:5173');
      expect(process.env._E2E_RUNTIME_ADMIN_WEB_URL).toBe('http://admin-web.test:5173');

      // 通过 require 重载 playwright.config.ts，断言 use.baseURL = 注入值
      const cfgPath = require.resolve(path.join(REPO_ROOT, 'playwright.config.ts'));
      delete require.cache[cfgPath];
      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const cfgMod = require(cfgPath);
      const cfg = cfgMod.default ?? cfgMod;
      expect(cfg.use?.baseURL).toBe('http://admin-web.test:5173');
    } finally {
      for (const k of keys) {
        if (backup[k] === undefined) delete process.env[k];
        else process.env[k] = backup[k];
      }
    }
  });
});

// ─────────────────────── U-11 fuzzy 拼写守护 ───────────────────────
test.describe('T-0000J U-11：@prod-safe 拼写守护（防灾难）', () => {
  test('U-11: 不出现 @prod_safe / @prodsafe / @prod-save / @prodSafe 等近似拼写', () => {
    const specs = listSpecFiles([SCRIPTS_DIR]);
    const fuzzy = [
      '@prod_safe',
      '@prodsafe',
      '@prod-save',
      '@prodSafe',
      '@Prod-safe',
      '@PROD-SAFE',
    ];
    const hits: string[] = [];
    for (const f of specs) {
      const t = fs.readFileSync(f, 'utf8');
      for (const pat of fuzzy) {
        if (t.includes(pat)) {
          hits.push(`${path.relative(REPO_ROOT, f)} → ${pat}`);
        }
      }
    }
    expect(hits, `fuzzy 命中：\n${hits.join('\n')}`).toEqual([]);
  });
});

// ─────────────────────── U-12 globalSetup writeProcessEnv 注入双 key ───────────────────────
test.describe('T-0000J U-12：writeProcessEnv 注入 ADMIN_WEB_URL + _E2E_RUNTIME_ADMIN_WEB_URL', () => {
  test('U-12: envLoader.ts writeProcessEnv 函数源码同时写入两个 key', () => {
    const txt = fs.readFileSync(path.join(REPO_ROOT, 'tests/scripts/support/envLoader.ts'), 'utf8');
    // 找 writeProcessEnv 函数体
    const idx = txt.indexOf('export function writeProcessEnv(');
    expect(idx).toBeGreaterThan(-1);
    const tail = txt.slice(idx);
    expect(tail).toContain('process.env.ADMIN_WEB_URL');
    expect(tail).toContain('process.env._E2E_RUNTIME_ADMIN_WEB_URL');
  });
});
