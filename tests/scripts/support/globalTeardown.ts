/**
 * T-0000H globalTeardown：仅 local profile 调用 reset-e2e.sh；失败仅 warn。
 */
import * as path from 'node:path';

import { loadE2EEnv } from './envLoader';
import { runShell as defaultRunShell, ShellExecError } from './runShell';

export interface TeardownDeps {
  runShell: (cmd: string, args: string[], opts: { env?: NodeJS.ProcessEnv; cwd?: string; timeoutMs?: number }) => Promise<{ exitCode: number; stderrTail: string[] }>;
  cwd: string;
  warn: (msg: string) => void;
}

export async function runGlobalTeardown(deps: TeardownDeps): Promise<void> {
  const ciTag = process.env.CI === 'true' ? '[E2E teardown ci]' : '[E2E teardown]';
  let profile: string;
  try {
    const env = loadE2EEnv({ cwd: deps.cwd });
    profile = env.profile;
  } catch (err) {
    deps.warn(`${ciTag} env load failed during teardown (non-fatal): ${(err as Error).message}`);
    return;
  }

  if (profile !== 'local') {
    // eslint-disable-next-line no-console
    console.log(`${ciTag} skip reset (profile=${profile}, remote profiles never auto-reset)`);
    return;
  }

  const resetFlag = process.env.E2E_RESET ?? '1';
  if (resetFlag === '0') {
    // eslint-disable-next-line no-console
    console.log(`${ciTag} skip reset (E2E_RESET=0)`);
    return;
  }

  try {
    await deps.runShell(
      'bash',
      [path.join(deps.cwd, 'scripts/dev/reset-e2e.sh'), '--yes'],
      {
        env: { ...process.env, E2E_PROFILE: 'local' },
        cwd: deps.cwd,
        timeoutMs: 30_000,
      },
    );
    // eslint-disable-next-line no-console
    console.log(`${ciTag} reset OK`);
  } catch (err) {
    const code = (err as ShellExecError).exitCode ?? 1;
    deps.warn(`${ciTag} reset failed (exit=${code}, non-fatal): ${(err as Error).message}`);
  }
}

export default async function globalTeardown(): Promise<void> {
  await runGlobalTeardown({
    runShell: defaultRunShell,
    cwd: process.cwd(),
    warn: (m) => console.warn(m),
  });
}
