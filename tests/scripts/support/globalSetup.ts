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
import { execSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import dotenv from 'dotenv';

import { loadE2EEnv, writeProcessEnv, sanitizeEnvForRuntimeJson, MissingEnvError } from './envLoader';
import type { E2EEnv } from './types';
import { runShell as defaultRunShell, ShellExecError } from './runShell';

const ANDROID_PKG = process.env.ANDROID_APP_ID ?? 'com.voice.room.android.local.debug';
const ANDROID_DEVICE_ID = process.env.ANDROID_DEVICE_ID ?? '9A251FFAZ00EAJ';
// 由 `adb shell cmd package resolve-activity --brief` 实测得到的 MainActivity 全限定名
const MAIN_ACTIVITY = `${ANDROID_PKG}/com.voice.room.android.presentation.MainActivity`;

async function androidWarmUp(): Promise<void> {
  try {
    execSync(`adb -s ${ANDROID_DEVICE_ID} get-state`, { stdio: 'pipe' });

    const pkgList = execSync(
      `adb -s ${ANDROID_DEVICE_ID} shell pm list packages ${ANDROID_PKG}`,
      { stdio: 'pipe' },
    ).toString();
    if (!pkgList.includes(ANDROID_PKG)) {
      console.warn(`[globalSetup] Android warm-up: ${ANDROID_PKG} not installed, skip`);
      return;
    }

    try {
      const dump = execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell pm dump ${ANDROID_PKG} | grep -E "stopped=|notLaunched="`,
        { stdio: 'pipe' },
      ).toString();
      console.log(`[globalSetup] Android app state: ${dump.trim()}`);
    } catch {
      // pm dump grep 失败无所谓，继续
    }

    execSync(
      `adb -s ${ANDROID_DEVICE_ID} shell am start --include-stopped-packages -n ${MAIN_ACTIVITY}`,
      { stdio: 'inherit' },
    );

    // 等待 App 完全启动（首次运行弹窗需要更长时间）
    await new Promise<void>((r) => setTimeout(r, 4000));

    // 尝试关闭「数据收集说明」等首次运行同意弹窗
    // 使用 uiautomator dump 定位「同意/确定/Accept/Agree/我已了解」按钮并点击
    try {
      const uiXml = execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell uiautomator dump /dev/stdout 2>/dev/null`,
        { stdio: 'pipe', timeout: 8000 },
      ).toString();

      const consentKeywords = ['同意', '确定', 'Accept', 'Agree', 'OK', '我已了解', '知道了'];
      // 匹配 <node ... text="同意" ... bounds="[x1,y1][x2,y2]" .../>
      const nodeRegex = /text="([^"]*)"[^/]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/g;
      let match;
      let dismissed = false;
      while ((match = nodeRegex.exec(uiXml)) !== null) {
        const text = match[1];
        if (consentKeywords.some((kw) => text.includes(kw))) {
          const cx = Math.floor((parseInt(match[2]) + parseInt(match[4])) / 2);
          const cy = Math.floor((parseInt(match[3]) + parseInt(match[5])) / 2);
          execSync(`adb -s ${ANDROID_DEVICE_ID} shell input tap ${cx} ${cy}`, { stdio: 'pipe' });
          await new Promise<void>((r) => setTimeout(r, 1000));
          console.log(`[globalSetup] Dismissed consent dialog: tapped "${text}" at (${cx},${cy})`);
          dismissed = true;
          break;
        }
      }
      if (!dismissed) {
        console.log('[globalSetup] No consent dialog detected, continuing');
      }
    } catch {
      // uiautomator 失败为非阻塞
      console.warn('[globalSetup] uiautomator dump failed (non-blocking), skipping consent check');
    }

    execSync(`adb -s ${ANDROID_DEVICE_ID} shell input keyevent KEYCODE_HOME`, { stdio: 'pipe' });
    await new Promise<void>((r) => setTimeout(r, 1000));

    console.log('[globalSetup] Android warm-up done – App is no longer in STOPPED state');
  } catch (e) {
    console.warn(`[globalSetup] Android warm-up failed (non-blocking): ${(e as Error).message}`);
  }
}

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

  // ── Step 0：Android App 暖机（Android 12+ STOPPED 状态下 monkey 无法启动）──
  // 在所有测试前，对真机目标包执行 am start --include-stopped-packages，
  // 脱离 STOPPED 状态，确保后续 Midscene agent.launch() 的 monkey 能成功。
  // 失败为 non-blocking（warn 不 throw），不影响 Web 端测试。
  await androidWarmUp();

  // 缺陷 1 修复 — CI 软门禁（与 .env.example CI_E2E_READY=0 注释语义对齐）：
  //   CI runner 默认未起 5 端依赖，preflight 必触退码 11~15。在显式开启 CI_E2E_READY=1
  //   之前（手动 workflow_dispatch 或长期方案 a 起服务的 job），让 CI 直接早退避免永红。
  //   本地 / staging / prod 实跑路径不受影响（CI 未置 'true'）。
  if (process.env.CI === 'true' && process.env.CI_E2E_READY !== '1') {
    // eslint-disable-next-line no-console
    console.log(`${ciTag} CI 软门禁未开启（CI_E2E_READY!=1），跳过 preflight/seed。如需在 CI 跑 E2E，请显式设置 secret CI_E2E_READY=1 + 起齐 5 端依赖。`);
    return;
  }

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

    // T-0000S：fail-fast 校验 USER_B / MUTED token + USER_MUTED_ID 已被 seed 注入。
    // 这三项是 26/29 SKIP-KNOWN 用例的解锁前提，缺失即认为 seed 失败。
    const requiredFromSeed = ['E2E_USER_B_TOKEN', 'E2E_MUTED_TOKEN', 'E2E_USER_MUTED_ID'] as const;
    const missingSeed = requiredFromSeed.filter((k) => !process.env[k]);
    if (missingSeed.length > 0) {
      process.stderr.write(`${ciTag} seed completed but missing required keys: ${missingSeed.join(', ')}\n`);
      deps.exit(22);
      throw new Error('seed missing required keys');
    }

    // T-0000S：诊断 redis-cli 容器化路径（不阻塞，仅日志）。
    try {
      // 延迟 require 以避免 unit-config 加载 globalSetup 时副作用
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const { resolveRedisCliMode, _resetRedisCliCacheForTest } = require('./redisCli') as typeof import('./redisCli');
      _resetRedisCliCacheForTest();
      const mode = resolveRedisCliMode(undefined, { useCache: true });
      // eslint-disable-next-line no-console
      console.log(`${ciTag} redis-cli mode = ${mode} (docker→native→unavailable)`);
      if (mode === 'unavailable') {
        process.stderr.write(`${ciTag} WARN: redis-cli unavailable; TC-AUTH/TC-WS/TC-RANKING redis-dependent cases will SKIP-KNOWN-FOLLOWUP\n`);
      }
    } catch (e) {
      process.stderr.write(`${ciTag} WARN: redisCli probe failed: ${(e as Error).message}\n`);
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
