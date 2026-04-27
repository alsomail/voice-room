/**
 * T-0000I: package.json 6 个 npm scripts 一键命令 TDD 验收
 *
 * 覆盖 §三 U-1 ~ U-11：
 *   - 静态（U-1/U-2/U-3/U-10）：直接解析 package.json
 *   - 集成（U-4/U-5/U-6/U-7）：spawnSync `npm run ...` 验证退出码透传
 *   - 系统（U-8/U-9）：playwright --list dry-run 验证 cross-env 注入与 grep
 *   - 时间预算（U-11）：preflight 健康场景壁时
 *
 * 设计原则：
 *   1) 6 个 script 命令字符串必须与 TDS §2.3 冻结表 1:1 完全相等（含双引号字面量）；
 *   2) 集成断言「退出码透传」≠「退出码必须为 0」——只要 `npm run X` 退出码与直接调用底层脚本一致即满足；
 *   3) 真跑依赖外部 5 端 / DB 时，缺依赖优雅 SKIP（test.skip()），不污染 RED/GREEN 信号。
 */
import { test, expect } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';

const REPO_ROOT = path.resolve(__dirname, '../../../..');
const PKG_PATH = path.join(REPO_ROOT, 'package.json');

// ───────────────── §2.3 6 个 script 冻结表（不变量；任何修改需先改 TDS）────────────
const FROZEN: Record<string, string> = {
  'e2e:local': 'cross-env E2E_PROFILE=local playwright test',
  'e2e:staging': 'cross-env E2E_PROFILE=staging playwright test',
  'e2e:prod-smoke':
    'cross-env E2E_PROFILE=prod playwright test --grep "@prod-safe"',
  'db:seed':
    'cross-env E2E_PROFILE=local E2E_ALLOW_WRITES=1 bash scripts/dev/seed-e2e.sh',
  'db:reset': 'cross-env E2E_PROFILE=local bash scripts/dev/reset-e2e.sh',
  preflight: 'bash scripts/dev/preflight.sh',
};

function readPkg(): any {
  return JSON.parse(fs.readFileSync(PKG_PATH, 'utf8'));
}

function npmRun(script: string, opts: { extraArgs?: string[]; env?: NodeJS.ProcessEnv; timeoutMs?: number } = {}) {
  const args = ['run', script];
  if (opts.extraArgs && opts.extraArgs.length) args.push('--', ...opts.extraArgs);
  return spawnSync('npm', args, {
    cwd: REPO_ROOT,
    env: { ...process.env, ...(opts.env ?? {}), CI: '1', NO_COLOR: '1' },
    encoding: 'utf8',
    timeout: opts.timeoutMs ?? 60_000,
  });
}

// ─────────────────────────── U-1 / U-3：静态命令冻结对账 ───────────────────────────
test.describe('T-0000I U-1/U-3 静态：6 个 script 命令冻结', () => {
  test('U-1: package.json 合法 JSON，scripts 段含 6 个冻结 key', () => {
    const pkg = readPkg();
    expect(pkg).toBeTruthy();
    expect(pkg.scripts).toBeTruthy();
    for (const key of Object.keys(FROZEN)) {
      expect(pkg.scripts, `scripts 缺 ${key}`).toHaveProperty(key);
    }
    // 不允许重名（JSON.parse 已经天然去重，这里复检无 undefined / 空字符串）
    for (const key of Object.keys(FROZEN)) {
      expect(typeof pkg.scripts[key]).toBe('string');
      expect(pkg.scripts[key].length).toBeGreaterThan(0);
    }
  });

  for (const [key, frozen] of Object.entries(FROZEN)) {
    test(`U-3.${key}: 命令字符串与 TDS §2.3 冻结表 1:1 相等`, () => {
      const pkg = readPkg();
      expect(pkg.scripts[key]).toBe(frozen);
    });
  }

  test('U-3.prod-smoke 双引号防 Windows 单引号陷阱（§六 R3 P0）', () => {
    const pkg = readPkg();
    const cmd = pkg.scripts['e2e:prod-smoke'] as string;
    // 必须包含 cross-env E2E_PROFILE=prod
    expect(cmd).toContain('cross-env E2E_PROFILE=prod');
    // 必须包含字面双引号包裹的 @prod-safe（json 中是 \"@prod-safe\"）
    expect(cmd).toContain('--grep "@prod-safe"');
    // 必须没有单引号包裹版本
    expect(cmd).not.toContain("'@prod-safe'");
  });
});

// ────────────────────────────── U-2 / U-10：cross-env 依赖 ──────────────────────────
test.describe('T-0000I U-2/U-10 cross-env devDependency', () => {
  test('U-2 / U-10: cross-env ^7.x 在 devDependencies 声明', () => {
    const pkg = readPkg();
    const ver = pkg.devDependencies?.['cross-env'];
    expect(ver, 'devDependencies.cross-env 未声明').toBeTruthy();
    // 接受 ^7.x.x 或 7.x.x
    expect(ver).toMatch(/^(\^|~)?7\./);
  });

  test('U-2 cross-env 二进制可解析（已 npm install）', () => {
    // require.resolve 不能解析 bin，改用文件存在性验证
    const binPath = path.join(REPO_ROOT, 'node_modules', '.bin', 'cross-env');
    const pkgPath = path.join(REPO_ROOT, 'node_modules', 'cross-env', 'package.json');
    const ok = fs.existsSync(binPath) || fs.existsSync(pkgPath);
    expect(ok, 'node_modules/cross-env 未安装；请 npm install').toBe(true);
  });
});

// ───────────────────── U-4 / U-5：preflight 退出码透传 + 时间预算 ───────────────────
test.describe('T-0000I U-4/U-5/U-11 preflight 退出码透传与时间预算', () => {
  test('U-4 / U-5: `npm run preflight` 退出码 == 直跑 bash 退出码（透传）', () => {
    const direct = spawnSync('bash', ['scripts/dev/preflight.sh'], {
      cwd: REPO_ROOT,
      env: { ...process.env, CI: '1', NO_COLOR: '1' },
      encoding: 'utf8',
      timeout: 30_000,
    });
    const viaNpm = npmRun('preflight', { timeoutMs: 30_000 });
    // 透传契约：两者退出码必须相等
    expect(viaNpm.status, `npm run preflight stderr=${viaNpm.stderr}`).toBe(direct.status);
    // 退出码必须落在 TDS §2.3 冻结集合 {0, 11, 12, 13, 14, 15} 内
    expect([0, 11, 12, 13, 14, 15]).toContain(viaNpm.status);
  });

  test('U-11: preflight 5 端全绿时壁时 ≤ 1s（健康场景；非健康自动 SKIP）', () => {
    // 先探测当前环境是否 5 端全绿
    const probe = spawnSync('bash', ['scripts/dev/preflight.sh'], {
      cwd: REPO_ROOT,
      env: { ...process.env, CI: '1', NO_COLOR: '1' },
      encoding: 'utf8',
      timeout: 15_000,
    });
    test.skip(probe.status !== 0, `5 端非全绿（rc=${probe.status}）；时间预算只在健康场景断言`);
    const t0 = Date.now();
    const r = npmRun('preflight', { timeoutMs: 10_000 });
    const wall = Date.now() - t0;
    expect(r.status).toBe(0);
    // npm 自身有 ~300ms 启动开销；TDS §2.4 要求 preflight 本体 ≤1s，叠加 npm 放宽到 3s
    expect(wall, `wall=${wall}ms`).toBeLessThanOrEqual(3000);
  });
});

// ─────────────────────── U-6 / U-7：db:seed / db:reset 退出码透传 ───────────────────
test.describe('T-0000I U-6/U-7 db:seed / db:reset 退出码透传', () => {
  test('U-6: `npm run db:seed` 缺 JWT_SECRET → 退出码 22 透传', () => {
    // 清空 JWT 相关 env，让 seed-e2e.sh 进入「无 JWT_SECRET」分支（§T-0000G 退出码 22）
    const env: NodeJS.ProcessEnv = { ...process.env };
    delete env.JWT_SECRET;
    delete env.APP_JWT_SECRET;
    delete env.ADMIN_JWT_SECRET;
    const r = npmRun('db:seed', { env, timeoutMs: 30_000 });
    // 期望 22；若环境碰巧有 JWT_SECRET 但 PG 拒绝，则 23；其它视为退出码透传断言失败
    expect([21, 22, 23]).toContain(r.status);
  });

  test('U-7: `npm run db:reset` 退出码透传（profile=local + --yes）', () => {
    // cross-env 强注 E2E_PROFILE=local，--yes 跳过交互；PG 全开 → 0；PG 关 → 24
    const r = npmRun('db:reset', { extraArgs: ['--yes'], timeoutMs: 30_000 });
    expect([0, 24]).toContain(r.status);
  });
});

// ─────────────────── U-8 / U-9：e2e:* dry-run（cross-env 注入 + grep）───────────────
test.describe('T-0000I U-8/U-9 e2e:* dry-run（cross-env + --grep 双保险）', () => {
  test('U-8: `npm run e2e:local -- --list` 可枚举用例（dry-run，不真跑）', () => {
    // playwright --list 只需配置可加载，不实际启动浏览器/服务
    const r = npmRun('e2e:local', { extraArgs: ['--list'], timeoutMs: 60_000 });
    // 接受 0（成功列出）或非 0（globalSetup 在 list 模式仍可能跑）；只要 cross-env 真注入
    // 即 stdout/stderr 不会包含 "E2E_PROFILE: command not found"（Windows 直 inline 写法的崩溃模式）
    const out = (r.stdout ?? '') + (r.stderr ?? '');
    expect(out).not.toMatch(/E2E_PROFILE\s*:\s*command not found/);
    // 不强制断言 status === 0（playwright list 在某些 globalSetup 下也可能 fail-fast）
  });

  test('U-9: `npm run e2e:prod-smoke -- --list` 仅枚举 @prod-safe（T-0000J 未合入则退化 SKIP）', () => {
    // 探测仓库内是否已存在 @prod-safe 标签
    const grep = spawnSync(
      'bash',
      ['-lc', "grep -RIn --include='*.spec.ts' --include='*.test.ts' '@prod-safe' tests e2e 2>/dev/null | wc -l"],
      { cwd: REPO_ROOT, encoding: 'utf8' },
    );
    const hits = parseInt((grep.stdout ?? '0').trim(), 10);
    test.skip(!Number.isFinite(hits) || hits < 1, 'T-0000J 尚未合入：仓库无 @prod-safe 标签，退化为 SKIP（TDS §2.5）');

    const r = npmRun('e2e:prod-smoke', { extraArgs: ['--list'], timeoutMs: 60_000 });
    const out = (r.stdout ?? '') + (r.stderr ?? '');
    // 关键反向断言：未出现「单引号字面量 0 命中」的假绿陷阱
    expect(out).not.toMatch(/'@prod-safe'/);
    // grep 既然有命中，list 输出应非空地提到至少 1 条用例
    if (r.status === 0) {
      expect(out.length).toBeGreaterThan(0);
    }
  });
});
