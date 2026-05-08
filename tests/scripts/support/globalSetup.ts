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
// P0-Fix Note: 经验证 com.voice.room.android.presentation.MainActivity 是正确路径
//   （com.voice.room.android.MainActivity 不存在，会导致 Error type 3）
const MAIN_ACTIVITY = `${ANDROID_PKG}/com.voice.room.android.presentation.MainActivity`;

/**
 * 尝试通过 uiautomator dump 关闭同意弹窗（最多重试 maxRetries 次）。
 * 返回 true 表示成功关闭或未检测到弹窗。
 */
async function dismissConsentDialogViaAdb(maxRetries = 3): Promise<boolean> {
  const consentKeywords = ['同意', '确定', 'Accept', 'Agree', 'OK', '我已了解', '知道了', 'موافقت', 'قبول'];
  // 匹配 <node ... text="同意" ... bounds="[x1,y1][x2,y2]" .../>
  const nodeRegex = /text="([^"]*)"[^/]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/g;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      // dump UI to file then pull (避免 /dev/stdout 的 pipe 截断问题)
      execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell uiautomator dump /sdcard/ui_warmup.xml`,
        { stdio: 'pipe', timeout: 8000 },
      );
      const uiXml = execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell cat /sdcard/ui_warmup.xml`,
        { stdio: 'pipe', timeout: 5000 },
      ).toString();

      let match;
      nodeRegex.lastIndex = 0;
      let dismissed = false;
      while ((match = nodeRegex.exec(uiXml)) !== null) {
        const text = match[1];
        if (consentKeywords.some((kw) => text.includes(kw))) {
          const cx = Math.floor((parseInt(match[2]) + parseInt(match[4])) / 2);
          const cy = Math.floor((parseInt(match[3]) + parseInt(match[5])) / 2);
          execSync(`adb -s ${ANDROID_DEVICE_ID} shell input tap ${cx} ${cy}`, { stdio: 'pipe' });
          await new Promise<void>((r) => setTimeout(r, 1500));
          console.log(`[globalSetup] Dismissed consent dialog (attempt ${attempt}): tapped "${text}" at (${cx},${cy})`);
          dismissed = true;
          break;
        }
      }
      if (!dismissed) {
        console.log(`[globalSetup] No consent dialog detected (attempt ${attempt}), continuing`);
        return true; // no dialog present
      }
    } catch {
      console.warn(`[globalSetup] uiautomator dump failed (attempt ${attempt}), skipping`);
      return true; // non-blocking
    }
    // 等待下一个弹窗（如果有多页）
    await new Promise<void>((r) => setTimeout(r, 1500));
  }
  return true;
}

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

    // force-stop 后重启，确保 App 从干净状态启动（避免残留 UI 状态影响测试）
    try {
      execSync(`adb -s ${ANDROID_DEVICE_ID} shell am force-stop ${ANDROID_PKG}`, { stdio: 'pipe' });
      await new Promise<void>((r) => setTimeout(r, 1000));
    } catch {
      // force-stop 失败无所谓，继续
    }

    execSync(
      `adb -s ${ANDROID_DEVICE_ID} shell am start --include-stopped-packages -n ${MAIN_ACTIVITY}`,
      { stdio: 'inherit' },
    );

    // 等待 App 完全启动（闪屏 + 初始化需要 5s）
    await new Promise<void>((r) => setTimeout(r, 5000));

    // 尝试多轮关闭「数据收集说明」等首次运行同意弹窗（支持多页弹窗）
    await dismissConsentDialogViaAdb(3);

    // P0-Fix：最终验证 — 等待 3s 再 dump XML，确认 App 已进入登录页
    // （检测手机号输入框或"获取验证码"/"登录"按钮，任意一个存在即视为成功）
    await new Promise<void>((r) => setTimeout(r, 3000));
    try {
      execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell uiautomator dump /sdcard/ui_final.xml`,
        { stdio: 'pipe', timeout: 8000 },
      );
      const finalXml = execSync(
        `adb -s ${ANDROID_DEVICE_ID} shell cat /sdcard/ui_final.xml`,
        { stdio: 'pipe', timeout: 5000 },
      ).toString();
      const loginKeywords = ['手机号', '登录', '获取验证码', 'phone', 'Login', 'Send Code', 'phoneInput', 'login'];
      const hasLoginScreen = loginKeywords.some((kw) => finalXml.includes(kw));
      if (hasLoginScreen) {
        console.log('[globalSetup] ✅ Final verification passed – login screen detected');
      } else {
        console.warn('[globalSetup] ⚠️ Final verification: login screen NOT detected in XML dump (App may be on unexpected screen). Tests may be unreliable.');
      }
    } catch {
      console.warn('[globalSetup] ⚠️ Final verification: uiautomator dump failed, skipping check');
    }

    // ❌ 移除：不再发送 KEYCODE_HOME —— 原实现会把 App 送到主屏幕背景，
    //    导致每个 AND 测试启动时 Midscene agent.launch() 看到主屏幕而非登录页。
    //    修复后让 App 保持前台，各 test 的 agent.launch() 会从当前前台状态接管。

    console.log('[globalSetup] Android warm-up done – App is in foreground, ready for tests');
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
          // RC-7 Fix: Inject JWT secrets from .env.<profile> (matches running server secrets).
          // Shell exports may carry stale values (e.g., test-admin-jwt-secret from root .env)
          // that differ from what e2e-up.sh injects into the servers. We overlay the profile
          // env file's JWT_SECRET / APP_JWT_SECRET / ADMIN_JWT_SECRET on top of process.env so
          // seed-e2e.sh always signs tokens with the same key the server uses for verification.
          env: (() => {
            const localEnvFile = path.join(deps.cwd, 'tests/scripts/env', `.env.${env.profile}`);
            const localVars = fs.existsSync(localEnvFile)
              ? dotenv.parse(fs.readFileSync(localEnvFile))
              : {};
            const jwtOverrides: Record<string, string> = {};
            for (const k of ['JWT_SECRET', 'APP_JWT_SECRET', 'ADMIN_JWT_SECRET']) {
              if (localVars[k]) jwtOverrides[k] = localVars[k];
            }
            return { ...process.env, ...jwtOverrides, E2E_PROFILE: env.profile, E2E_ALLOW_WRITES: '1' };
          })(),
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
