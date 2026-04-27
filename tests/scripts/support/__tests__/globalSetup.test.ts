/**
 * globalSetup / globalTeardown 集成测试（T-0000H §2.8.2）
 *
 * 通过依赖注入（runShell stub）验证：
 *   1. 子进程调用顺序与命令；2. 退出码透传；3. seed-output 注入 process.env；
 *   4. teardown profile=staging skip；5. teardown 失败仅 warn。
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { runGlobalSetup, type SetupDeps } from '../globalSetup';
import { runGlobalTeardown, type TeardownDeps } from '../globalTeardown';
import type { E2EEnv } from '../types';

interface ShellCall { cmd: string; args: string[]; env?: NodeJS.ProcessEnv; cwd?: string }

function makeStubShell(seq: Array<{ exit: number; stderrTail?: string[]; onCall?: (call: ShellCall) => void }>) {
  const calls: ShellCall[] = [];
  let idx = 0;
  return {
    calls,
    runShell: async (cmd: string, args: string[], opts: { env?: NodeJS.ProcessEnv; cwd?: string }) => {
      const call: ShellCall = { cmd, args, env: opts.env, cwd: opts.cwd };
      calls.push(call);
      const item = seq[idx++] ?? { exit: 0 };
      item.onCall?.(call);
      if (item.exit !== 0) {
        const err: NodeJS.ErrnoException & { exitCode?: number; stderrTail?: string[] } = new Error(
          `[shell] ${cmd} exited ${item.exit}`
        );
        err.exitCode = item.exit;
        err.stderrTail = item.stderrTail ?? [];
        throw err;
      }
      return { exitCode: 0, stderrTail: [] };
    },
  };
}

const LOCAL_FULL: Record<string, string> = {
  E2E_PROFILE: 'local',
  E2E_ALLOW_WRITES: '1',
  APP_SERVER_BASE_URL: 'http://localhost:3000',
  ADMIN_SERVER_BASE_URL: 'http://localhost:3001',
  ADMIN_WEB_URL: 'http://localhost:5173',
  APP_WS_URL: 'ws://localhost:3000/ws',
  DATABASE_URL: 'postgres://u:p@localhost:5432/v',
  REDIS_URL: 'redis://localhost:6379',
  ANDROID_APP_ID: 'com.x',
  E2E_VALID_TOKEN: 't1',
  E2E_EXPIRED_TOKEN: 't2',
  E2E_ADMIN_TOKEN: 't3',
  E2E_OP_TOKEN: 't4',
  E2E_CS_TOKEN: 't5',
  E2E_FIN_TOKEN: 't6',
  E2E_EXPIRED_ADMIN_TOKEN: 't7',
  E2E_ROOM_ID: 'r',
  E2E_USER_A_ID: 'a',
  E2E_USER_B_ID: 'b',
};

const STAGING_FULL: Record<string, string> = {
  ...LOCAL_FULL,
  E2E_PROFILE: 'staging',
  APP_SERVER_BASE_URL: 'https://stg.example.com',
  DATABASE_URL: '',
  REDIS_URL: '',
};
const PROD_FULL: Record<string, string> = { ...STAGING_FULL, E2E_PROFILE: 'prod', E2E_ALLOW_WRITES: '0' };

const ENV_KEYS = [...Object.keys(LOCAL_FULL), 'E2E_SEED', 'E2E_RESET', 'CI'];

function prepareCwd(profile: string, content: Record<string, string>): string {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'gsetup-'));
  fs.mkdirSync(path.join(tmp, 'tests', 'scripts', 'env'), { recursive: true });
  fs.mkdirSync(path.join(tmp, 'scripts', 'dev'), { recursive: true });
  const lines = Object.entries(content).map(([k, v]) => `${k}=${v}`).join('\n');
  fs.writeFileSync(path.join(tmp, 'tests', 'scripts', 'env', `.env.${profile}`), lines);
  return tmp;
}

function snap() { return { ...process.env }; }
function restore(s: NodeJS.ProcessEnv) {
  for (const k of ENV_KEYS) delete process.env[k];
  for (const [k, v] of Object.entries(s)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
}

test.describe('globalSetup', () => {
  let s: NodeJS.ProcessEnv;
  test.beforeEach(() => { s = snap(); for (const k of ENV_KEYS) delete process.env[k]; });
  test.afterEach(() => restore(s));

  test('local 全链路：preflight 0 + seed 0 + .e2e-runtime.json 写入', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([
      { exit: 0 },                                    // preflight
      { exit: 0, onCall: () => {                      // seed → 写 .seed-output.env
        fs.writeFileSync(
          path.join(cwd, 'scripts', 'dev', '.seed-output.env'),
          'E2E_VALID_TOKEN=seeded-tok-valid\nE2E_ROOM_ID=seeded-room\nE2E_USER_A_ID=seeded-a\nE2E_USER_B_ID=seeded-b\nE2E_EXPIRED_TOKEN=seeded-exp\n',
        );
      } },
    ]);
    let exited: number | null = null;
    const deps: SetupDeps = { runShell: stub.runShell, exit: (c) => { exited = c; throw new Error('exit'); }, cwd };
    await runGlobalSetup(deps);

    expect(exited).toBeNull();
    expect(stub.calls).toHaveLength(2);
    expect(stub.calls[0].cmd).toBe('bash');
    expect(stub.calls[0].args[0]).toContain('preflight.sh');
    expect(stub.calls[0].args).toContain('--profile');
    expect(stub.calls[0].args).toContain('local');
    expect(stub.calls[1].args[0]).toContain('seed-e2e.sh');
    // process.env 注入
    expect(process.env.E2E_VALID_TOKEN).toBe('seeded-tok-valid');
    expect(process.env.E2E_ROOM_ID).toBe('seeded-room');
    // runtime json
    const runtime = JSON.parse(fs.readFileSync(path.join(cwd, 'tests', 'scripts', '.e2e-runtime.json'), 'utf8'));
    expect(runtime.profile).toBe('local');
    expect(runtime.tokens.valid).toBe('seeded-tok-valid');
  });

  test('preflight 退出码 13 → setup exit 13', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([{ exit: 13, stderrTail: ['app server unreachable'] }]);
    let exited: number | null = null;
    const deps: SetupDeps = { runShell: stub.runShell, exit: (c) => { exited = c; throw new Error('exit'); }, cwd };
    await expect(runGlobalSetup(deps)).rejects.toThrow();
    expect(exited).toBe(13);
    expect(stub.calls).toHaveLength(1); // preflight 失败后不调 seed
  });

  test('seed 退出码 22 → setup exit 22', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([{ exit: 0 }, { exit: 22, stderrTail: ['JWT_SECRET missing'] }]);
    let exited: number | null = null;
    const deps: SetupDeps = { runShell: stub.runShell, exit: (c) => { exited = c; throw new Error('exit'); }, cwd };
    await expect(runGlobalSetup(deps)).rejects.toThrow();
    expect(exited).toBe(22);
  });

  test('staging profile 不调 seed（仅 preflight）', async () => {
    const cwd = prepareCwd('staging', STAGING_FULL);
    process.env.E2E_PROFILE = 'staging';
    const stub = makeStubShell([{ exit: 0 }]);
    const deps: SetupDeps = { runShell: stub.runShell, exit: () => { throw new Error('x'); }, cwd };
    await runGlobalSetup(deps);
    expect(stub.calls).toHaveLength(1);
    expect(stub.calls[0].args[0]).toContain('preflight.sh');
  });

  test('local + E2E_SEED=0 不调 seed', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    process.env.E2E_SEED = '0';
    const stub = makeStubShell([{ exit: 0 }]);
    const deps: SetupDeps = { runShell: stub.runShell, exit: () => { throw new Error('x'); }, cwd };
    await runGlobalSetup(deps);
    expect(stub.calls).toHaveLength(1);
  });

  test('prod profile + allowWrites=0 不调 seed', async () => {
    const cwd = prepareCwd('prod', PROD_FULL);
    process.env.E2E_PROFILE = 'prod';
    const stub = makeStubShell([{ exit: 0 }]);
    const deps: SetupDeps = { runShell: stub.runShell, exit: () => { throw new Error('x'); }, cwd };
    await runGlobalSetup(deps);
    expect(stub.calls).toHaveLength(1);
  });

  test('envLoader 缺字段 → exit 78（不调 spawn）', async () => {
    const m = { ...STAGING_FULL };
    delete m.APP_SERVER_BASE_URL;
    const cwd = prepareCwd('staging', m);
    process.env.E2E_PROFILE = 'staging';
    const stub = makeStubShell([]);
    let exited: number | null = null;
    const deps: SetupDeps = { runShell: stub.runShell, exit: (c) => { exited = c; throw new Error('exit'); }, cwd };
    await expect(runGlobalSetup(deps)).rejects.toThrow();
    expect(exited).toBe(78);
    expect(stub.calls).toHaveLength(0);
  });

  test('preflight 透传 env（含 E2E_PROFILE）', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([{ exit: 0 }, { exit: 0 }]);
    const deps: SetupDeps = { runShell: stub.runShell, exit: () => { throw new Error('x'); }, cwd };
    await runGlobalSetup(deps);
    expect(stub.calls[0].env?.E2E_PROFILE).toBe('local');
  });
});

test.describe('globalTeardown', () => {
  let s: NodeJS.ProcessEnv;
  test.beforeEach(() => { s = snap(); for (const k of ENV_KEYS) delete process.env[k]; });
  test.afterEach(() => restore(s));

  test('local + E2E_RESET=1（默认）→ spawn reset-e2e.sh', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([{ exit: 0 }]);
    const warns: string[] = [];
    const deps: TeardownDeps = { runShell: stub.runShell, cwd, warn: (m) => warns.push(m) };
    await runGlobalTeardown(deps);
    expect(stub.calls).toHaveLength(1);
    expect(stub.calls[0].args[0]).toContain('reset-e2e.sh');
    expect(stub.calls[0].args).toContain('--yes');
  });

  test('staging profile 不调 reset', async () => {
    const cwd = prepareCwd('staging', STAGING_FULL);
    process.env.E2E_PROFILE = 'staging';
    const stub = makeStubShell([]);
    const deps: TeardownDeps = { runShell: stub.runShell, cwd, warn: () => {} };
    await runGlobalTeardown(deps);
    expect(stub.calls).toHaveLength(0);
  });

  test('local + E2E_RESET=0 不调 reset', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    process.env.E2E_RESET = '0';
    const stub = makeStubShell([]);
    const deps: TeardownDeps = { runShell: stub.runShell, cwd, warn: () => {} };
    await runGlobalTeardown(deps);
    expect(stub.calls).toHaveLength(0);
  });

  test('reset 失败仅 warn，不抛', async () => {
    const cwd = prepareCwd('local', LOCAL_FULL);
    process.env.E2E_PROFILE = 'local';
    const stub = makeStubShell([{ exit: 24, stderrTail: ['no connection'] }]);
    const warns: string[] = [];
    const deps: TeardownDeps = { runShell: stub.runShell, cwd, warn: (m) => warns.push(m) };
    await runGlobalTeardown(deps); // 不应 throw
    expect(warns.length).toBeGreaterThan(0);
    expect(warns.join('\n')).toContain('reset failed');
  });
});
