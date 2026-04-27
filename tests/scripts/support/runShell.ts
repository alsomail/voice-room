/**
 * runShell：spawn 子进程封装，统一退出码 / stderr 截尾 / 超时。
 * T-0000H §2.4.3 规范。
 */
import { spawn, type SpawnOptions } from 'node:child_process';

export interface RunShellOpts {
  env?: NodeJS.ProcessEnv;
  cwd?: string;
  /** 默认 60_000 ms。 */
  timeoutMs?: number;
  /** 如果为 true，将 stdio inherit；否则 capture stderr 末 20 行。 */
  inheritStdio?: boolean;
}

export interface RunShellResult {
  exitCode: number;
  stderrTail: string[];
}

export class ShellExecError extends Error {
  public readonly exitCode: number;
  public readonly stderrTail: string[];
  constructor(cmd: string, exitCode: number, stderrTail: string[]) {
    super(`[shell] ${cmd} exited ${exitCode}`);
    this.name = 'ShellExecError';
    this.exitCode = exitCode;
    this.stderrTail = stderrTail;
  }
}

const TAIL_LINES = 20;

/**
 * 执行 shell 命令；非 0 退出码 → 抛 ShellExecError（携带 exitCode）。
 * stdio: stdout inherit；stderr 同时 inherit（实时打印）+ 内存留尾。
 */
export async function runShell(
  cmd: string,
  args: string[],
  opts: RunShellOpts = {},
): Promise<RunShellResult> {
  const timeoutMs = opts.timeoutMs ?? 60_000;
  return new Promise((resolve, reject) => {
    const stderrTail: string[] = [];
    const spawnOpts: SpawnOptions = {
      cwd: opts.cwd ?? process.cwd(),
      env: opts.env ?? process.env,
      stdio: ['ignore', 'inherit', 'pipe'],
    };
    const child = spawn(cmd, args, spawnOpts);
    let killedByTimeout = false;
    const timer = setTimeout(() => {
      killedByTimeout = true;
      child.kill('SIGKILL');
    }, timeoutMs);

    child.stderr?.on('data', (chunk: Buffer) => {
      const text = chunk.toString('utf8');
      // 实时透传到当前 stderr
      process.stderr.write(text);
      for (const line of text.split('\n')) {
        if (line.length === 0) continue;
        stderrTail.push(line);
        if (stderrTail.length > TAIL_LINES) stderrTail.shift();
      }
    });

    child.on('error', (err) => {
      clearTimeout(timer);
      reject(err);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      const exitCode = code ?? (signal ? 1 : 1);
      if (killedByTimeout) {
        reject(new ShellExecError(`${cmd} ${args.join(' ')}`, 124, [
          ...stderrTail,
          `[runShell] timeout after ${timeoutMs}ms`,
        ]));
        return;
      }
      if (exitCode !== 0) {
        reject(new ShellExecError(`${cmd} ${args.join(' ')}`, exitCode, stderrTail));
        return;
      }
      resolve({ exitCode: 0, stderrTail });
    });
  });
}
