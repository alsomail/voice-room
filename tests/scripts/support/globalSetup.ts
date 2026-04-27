/**
 * T-0000H globalSetup：Playwright 启动期单一编排入口。
 *
 * 流程（TDS §2.4）：
 *   Step 1 envLoader → fail-fast（缺字段退出 78）
 *   Step 2 preflight  → 退出 11~15 透传
 *   Step 3 seed       → 仅 local && allowWrites && E2E_SEED!=0；退出 21~24 透传
 *   Step 4 writeProcessEnv（worker 可读）
 *   Step 5 持久化 .e2e-runtime.json（fixture 跨进程读）
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import dotenv from 'dotenv';

import { loadE2EEnv, writeProcessEnv, sanitizeEnvForRuntimeJson, MissingEnvError } from './envLoader';
import type { E2EEnv } from './types';
import { runShell as defaultRunShell, ShellExecError } from './runShell';

export interface SetupDeps {
  /** 可注入的 shell runner（测试时使用 stub）。 */
  runShell: (cmd: string, args: string[], opts: { env?: NodeJS.ProcessEnv; cwd?: string; timeoutMs?: number }) => Promise<{ exitCode: number; stderrTail: string[] }>;
  /** 进程退出函数（测试时拦截）。 */
  exit: (code: number) => never;
  /** 仓库根目录。 */
  cwd: string;
}

/** 实现核心：可注入依赖，便于单测。失败时调用 deps.exit(<rc>) 后再抛错保证调用方流程终止。 */
export async function runGlobalSetup(deps: SetupDeps): Promise<void> {
  const t0 = Date.now();
  const ciTag = process.env.CI === 'true' ? '[E2E setup ci]' : '[E2E setup]';

  let env: E2EEnv;
  // ── Step 1：envLoader ──
  try {
    env = loadE2EEnv({ cwd: deps.cwd });
  } catch (err) {
    const code = (err as MissingEnvError).exitCode ?? 1;
    process.stderr.write(`${ciTag} env load failed:\n${(err as Error).message}\n`);
    deps.exit(code);
    throw err;
  }
  // eslint-disable-next-line no-console
  console.log(`${ciTag} profile=${env.profile} allowWrites=${env.allowWrites}`);

  // ── Step 2：preflight ──
  try {
    await deps.runShell(
      'bash',
      [path.join(deps.cwd, 'scripts/dev/preflight.sh'), '--profile', env.profile],
      {
        env: { ...process.env, E2E_PROFILE: env.profile },
        cwd: deps.cwd,
        timeoutMs: 60_000,
      },
    );
  } catch (err) {
    const code = (err as ShellExecError).exitCode ?? 1;
    const tail = (err as ShellExecError).stderrTail ?? [];
    process.stderr.write(`${ciTag} preflight failed (exit=${code})\n`);
    if (tail.length) process.stderr.write(tail.slice(-20).join('\n') + '\n');
    deps.exit(code);
    throw err;
  }

  // ── Step 3：seed（条件执行） ──
  const seedFlag = process.env.E2E_SEED ?? '1';
  const shouldSeed = env.profile === 'local' && env.allowWrites && seedFlag !== '0';
  if (shouldSeed) {
    try {
      await deps.runShell(
        'bash',
        [path.join(deps.cwd, 'scripts/dev/seed-e2e.sh')],
        {
          env: { ...process.env, E2E_PROFILE: env.profile, E2E_ALLOW_WRITES: '1' },
          cwd: deps.cwd,
          timeoutMs: 60_000,
        },
      );
    } catch (err) {
      const code = (err as ShellExecError).exitCode ?? 1;
      const tail = (err as ShellExecError).stderrTail ?? [];
      process.stderr.write(`${ciTag} seed failed (exit=${code})\n`);
      if (tail.length) process.stderr.write(tail.slice(-20).join('\n') + '\n');
      deps.exit(code);
      throw err;
    }

    // 解析 .seed-output.env → 注入 process.env → 重算 env
    const seedOut = path.join(deps.cwd, 'scripts/dev/.seed-output.env');
    if (fs.existsSync(seedOut)) {
      const parsed = dotenv.parse(fs.readFileSync(seedOut));
      for (const [k, v] of Object.entries(parsed)) process.env[k] = v;
      env = loadE2EEnv({ cwd: deps.cwd });
    }
  }

  // ── Step 4：writeProcessEnv ──
  writeProcessEnv(env);

  // ── Step 5：持久化 .e2e-runtime.json（T-0000K §2.7：脱敏后写盘，API Key 永不落盘）──
  const runtimePath = path.join(deps.cwd, 'tests/scripts/.e2e-runtime.json');
  fs.mkdirSync(path.dirname(runtimePath), { recursive: true });
  fs.writeFileSync(runtimePath, JSON.stringify(sanitizeEnvForRuntimeJson(env), null, 2), { mode: 0o600 });

  // eslint-disable-next-line no-console
  console.log(`${ciTag} OK in ${Date.now() - t0}ms`);
}

/** Playwright 默认导出入口（生产路径，使用真实 spawn）。 */
export default async function globalSetup(): Promise<void> {
  await runGlobalSetup({
    runShell: defaultRunShell,
    exit: (code) => process.exit(code),
    cwd: process.cwd(),
  });
}
