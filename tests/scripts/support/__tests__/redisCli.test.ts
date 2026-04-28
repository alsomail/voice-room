/**
 * T-0000S — redisCli helper 单元测试。
 *
 * 行为契约（TDS §修订后方案 / Step 2）：
 *   1. 优先 `docker exec vr-redis redis-cli ...`；
 *   2. docker 不可用时回退系统 PATH 中的 `redis-cli`；
 *   3. 都不可用时 isRedisCliAvailable() === false 且 redisExec 抛 RedisCliUnavailableError；
 *   4. 通过依赖注入（execSync stub）覆盖三种路径，避免污染主机环境。
 */
import { test, expect } from '@playwright/test';

import {
  buildRedisCliCommand,
  RedisCliUnavailableError,
  type RedisCliDeps,
  resolveRedisCliMode,
  redisExecWithDeps,
  isRedisCliAvailableWithDeps,
} from '../redisCli';

interface ExecCall { cmd: string; opts?: Record<string, unknown> }

function makeStub(impl: (cmd: string) => string | Error) {
  const calls: ExecCall[] = [];
  const fn = (cmd: string, opts?: Record<string, unknown>): Buffer | string => {
    calls.push({ cmd, opts });
    const r = impl(cmd);
    if (r instanceof Error) throw r;
    return r;
  };
  return { calls, fn };
}

test.describe('redisCli', () => {
  test('buildRedisCliCommand: docker 模式拼成 docker exec vr-redis redis-cli ARGS', () => {
    const cmd = buildRedisCliCommand('docker', ['SETEX', 'k', '60', '1']);
    expect(cmd).toBe('docker exec vr-redis redis-cli SETEX k 60 1');
  });

  test('buildRedisCliCommand: native 模式拼成 redis-cli ARGS', () => {
    const cmd = buildRedisCliCommand('native', ['PING']);
    expect(cmd).toBe('redis-cli PING');
  });

  test('buildRedisCliCommand: 含特殊字符的 args 走 shell 单引号转义', () => {
    const cmd = buildRedisCliCommand('docker', ['SET', 'foo', "it's"]);
    // 单引号闭合 → 重开 → 不允许裸 ' 出现在最终命令字符串中
    expect(cmd).toContain("docker exec vr-redis redis-cli");
    expect(cmd.endsWith("SET foo 'it'\\''s'")).toBe(true);
  });

  test('resolveRedisCliMode: docker 探测成功 → docker 模式', () => {
    const stub = makeStub((cmd) => {
      if (cmd.startsWith('docker exec vr-redis')) return Buffer.from('PONG\n');
      return new Error('redis-cli not found');
    });
    const deps: RedisCliDeps = { execSync: stub.fn as RedisCliDeps['execSync'] };
    expect(resolveRedisCliMode(deps)).toBe('docker');
    // 仅探测过 docker，不应再探系统 redis-cli
    expect(stub.calls.some((c) => c.cmd.startsWith('docker exec vr-redis'))).toBe(true);
  });

  test('resolveRedisCliMode: docker 失败 + 系统 redis-cli 成功 → native 模式', () => {
    const stub = makeStub((cmd) => {
      if (cmd.startsWith('docker exec')) return new Error('no such container');
      if (cmd.startsWith('redis-cli')) return Buffer.from('redis-cli 7.0.0\n');
      return new Error('unexpected');
    });
    const deps: RedisCliDeps = { execSync: stub.fn as RedisCliDeps['execSync'] };
    expect(resolveRedisCliMode(deps)).toBe('native');
  });

  test('resolveRedisCliMode: 都不可用 → unavailable', () => {
    const stub = makeStub(() => new Error('boom'));
    const deps: RedisCliDeps = { execSync: stub.fn as RedisCliDeps['execSync'] };
    expect(resolveRedisCliMode(deps)).toBe('unavailable');
  });

  test('isRedisCliAvailableWithDeps: 综合三种状态', () => {
    const okDocker = makeStub((c) => (c.startsWith('docker') ? 'PONG' : new Error('x')));
    const okNative = makeStub((c) => (c.startsWith('docker') ? new Error('x') : 'PONG'));
    const noNo = makeStub(() => new Error('x'));
    expect(isRedisCliAvailableWithDeps({ execSync: okDocker.fn as RedisCliDeps['execSync'] })).toBe(true);
    expect(isRedisCliAvailableWithDeps({ execSync: okNative.fn as RedisCliDeps['execSync'] })).toBe(true);
    expect(isRedisCliAvailableWithDeps({ execSync: noNo.fn as RedisCliDeps['execSync'] })).toBe(false);
  });

  test('redisExecWithDeps: docker 模式调用并返回 trimmed stdout', async () => {
    const stub = makeStub((cmd) => {
      if (cmd === 'docker exec vr-redis redis-cli --version') return Buffer.from('redis-cli 7\n');
      if (cmd === 'docker exec vr-redis redis-cli PING') return Buffer.from('PONG\n');
      return new Error('unexpected: ' + cmd);
    });
    const out = await redisExecWithDeps(['PING'], { execSync: stub.fn as RedisCliDeps['execSync'] });
    expect(out).toBe('PONG');
    expect(stub.calls.map((c) => c.cmd)).toContain('docker exec vr-redis redis-cli PING');
  });

  test('redisExecWithDeps: native 回退路径', async () => {
    const stub = makeStub((cmd) => {
      if (cmd.startsWith('docker')) return new Error('no docker');
      if (cmd === 'redis-cli --version') return 'redis-cli 7';
      if (cmd === 'redis-cli PING') return 'PONG';
      return new Error('unexpected: ' + cmd);
    });
    const out = await redisExecWithDeps(['PING'], { execSync: stub.fn as RedisCliDeps['execSync'] });
    expect(out).toBe('PONG');
  });

  test('redisExecWithDeps: 不可用时抛 RedisCliUnavailableError', async () => {
    const stub = makeStub(() => new Error('nope'));
    await expect(
      redisExecWithDeps(['PING'], { execSync: stub.fn as RedisCliDeps['execSync'] }),
    ).rejects.toBeInstanceOf(RedisCliUnavailableError);
  });
});
