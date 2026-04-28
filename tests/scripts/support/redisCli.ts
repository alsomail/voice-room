/**
 * T-0000S — redisCli helper：容器化 redis-cli 自动注入。
 *
 * 优先级：
 *   1) `docker exec vr-redis redis-cli ...`（docker-compose 起的 vr-redis 容器）
 *   2) 本地 PATH 中的 `redis-cli`（fallback；macOS 用户通常 brew install redis）
 *   3) 都不可用 → isRedisCliAvailable() === false 且 redisExec() 抛 RedisCliUnavailableError
 *      → 调用方可据此 graceful skip 而非 fail。
 *
 * 设计要点：
 *   - 同步 execSync 走子进程，但对外暴露 async API（与 TDS Plan §修订方案 一致）。
 *   - 探测结果按进程缓存（避免每次都重新 docker exec），同时保留 deps 入参以便单测。
 *   - shell 转义：所有 args 用单引号包裹并转义内嵌 ' → '\''。
 */
import { execSync as defaultExecSync } from 'node:child_process';

export type RedisCliMode = 'docker' | 'native' | 'unavailable';

export class RedisCliUnavailableError extends Error {
  constructor(message: string = 'redis-cli unavailable: neither `docker exec vr-redis redis-cli` nor system `redis-cli` is callable') {
    super(message);
    this.name = 'RedisCliUnavailableError';
  }
}

export interface RedisCliDeps {
  /** 同步 execSync（child_process.execSync 接口的子集）。 */
  execSync: (cmd: string, opts?: { stdio?: unknown; encoding?: BufferEncoding; timeout?: number }) => Buffer | string;
}

const DEFAULT_DEPS: RedisCliDeps = { execSync: defaultExecSync };

const DOCKER_PREFIX = 'docker exec vr-redis redis-cli';
const NATIVE_PREFIX = 'redis-cli';

/** POSIX shell 单引号转义（不依赖外部库，覆盖单引号、空格、$、`）。 */
function shellQuote(arg: string): string {
  if (/^[A-Za-z0-9_:\.\-\/]+$/.test(arg)) return arg;
  return `'${arg.replace(/'/g, `'\\''`)}'`;
}

export function buildRedisCliCommand(mode: 'docker' | 'native', args: string[]): string {
  const prefix = mode === 'docker' ? DOCKER_PREFIX : NATIVE_PREFIX;
  const escaped = args.map(shellQuote).join(' ');
  return escaped ? `${prefix} ${escaped}` : prefix;
}

function probe(deps: RedisCliDeps, cmd: string): boolean {
  try {
    deps.execSync(cmd, { stdio: 'pipe', timeout: 5000 });
    return true;
  } catch {
    return false;
  }
}

let cachedMode: RedisCliMode | null = null;

export function resolveRedisCliMode(deps: RedisCliDeps = DEFAULT_DEPS, opts: { useCache?: boolean } = {}): RedisCliMode {
  if (opts.useCache && cachedMode) return cachedMode;
  let mode: RedisCliMode;
  if (probe(deps, `${DOCKER_PREFIX} --version`)) {
    mode = 'docker';
  } else if (probe(deps, `${NATIVE_PREFIX} --version`)) {
    mode = 'native';
  } else {
    mode = 'unavailable';
  }
  if (opts.useCache) cachedMode = mode;
  return mode;
}

export function isRedisCliAvailableWithDeps(deps: RedisCliDeps = DEFAULT_DEPS): boolean {
  return resolveRedisCliMode(deps) !== 'unavailable';
}

/** 进程级缓存版本（生产路径；首次解析后复用）。 */
export function isRedisCliAvailable(): boolean {
  return resolveRedisCliMode(DEFAULT_DEPS, { useCache: true }) !== 'unavailable';
}

export async function redisExecWithDeps(args: string[], deps: RedisCliDeps = DEFAULT_DEPS): Promise<string> {
  const mode = resolveRedisCliMode(deps);
  if (mode === 'unavailable') {
    throw new RedisCliUnavailableError();
  }
  const cmd = buildRedisCliCommand(mode, args);
  try {
    const raw = deps.execSync(cmd, { encoding: 'utf-8', timeout: 10_000 });
    const text = typeof raw === 'string' ? raw : raw.toString('utf-8');
    return text.trim();
  } catch (err) {
    // execSync 抛非 0 退出 → 暴露给调用方判断（不视为 unavailable）。
    throw err;
  }
}

/** 生产路径：缓存 mode；调用方只关心 args + 返回。 */
export async function redisExec(args: string[]): Promise<string> {
  const mode = resolveRedisCliMode(DEFAULT_DEPS, { useCache: true });
  if (mode === 'unavailable') throw new RedisCliUnavailableError();
  const cmd = buildRedisCliCommand(mode, args);
  const raw = defaultExecSync(cmd, { encoding: 'utf-8', timeout: 10_000 });
  return raw.trim();
}

/** 同步包装（兼容旧的 inline `redis(cmd)` 调用点）。仅在已确认 isRedisCliAvailable() 后使用。 */
export function redisExecSync(args: string[]): string {
  const mode = resolveRedisCliMode(DEFAULT_DEPS, { useCache: true });
  if (mode === 'unavailable') throw new RedisCliUnavailableError();
  const cmd = buildRedisCliCommand(mode, args);
  return defaultExecSync(cmd, { encoding: 'utf-8', timeout: 10_000 }).trim();
}

/** 测试 hook：清缓存。 */
export function _resetRedisCliCacheForTest(): void {
  cachedMode = null;
}
