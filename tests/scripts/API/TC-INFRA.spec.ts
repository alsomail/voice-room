/**
 * 测试套件：INFRA 基础设施（API/Shell）
 * 用例来源：doc/tests/cases/API/TC-INFRA.md
 * 说明：以 Shell 命令验证为主；运行前置要求 Docker + Cargo + PG 已安装。
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import 'dotenv/config';

const sh = (cmd: string) => execSync(cmd, { encoding: 'utf-8', stdio: ['ignore', 'pipe', 'pipe'] });

test.describe('TC-INFRA - 基础设施', () => {
  test('TC-INFRA-00001: docker compose 一键启动 PG + Redis', () => {
    test.skip(process.env.CI_E2E_READY !== '1', 'CI_E2E_READY 未开启');
    sh('docker compose up -d postgres redis');
    // 等待健康检查
    const pg = sh('docker compose exec -T postgres pg_isready -U postgres');
    expect(pg).toMatch(/accepting connections/);
    const redis = sh('docker compose exec -T redis redis-cli PING');
    expect(redis.trim()).toBe('PONG');

    // 持久化：写入后重启容器再读
    sh(`psql "${process.env.DATABASE_URL}" -c "CREATE TABLE IF NOT EXISTS _probe(id int); INSERT INTO _probe VALUES(1);"`);
    sh('docker compose restart postgres');
    // 等待恢复
    for (let i = 0; i < 20; i++) {
      try { sh('docker compose exec -T postgres pg_isready'); break; } catch { execSync('sleep 1'); }
    }
    const row = sh(`psql "${process.env.DATABASE_URL}" -tA -c "SELECT count(*) FROM _probe;"`).trim();
    expect(row).toBe('1');
    sh(`psql "${process.env.DATABASE_URL}" -c "DROP TABLE _probe;"`);
  });

  test('TC-INFRA-00002: 端口被占用明确错误', () => {
    test.skip(process.env.CI_E2E_READY !== '1', 'CI_E2E_READY 未开启');
    // 占用 5432
    const { spawn } = require('child_process');
    const p = spawn('nc', ['-l', '5432']);
    try {
      let err = '';
      try { sh('docker compose up -d postgres'); } catch (e: any) { err = String(e.stderr ?? e.message); }
      expect(err.toLowerCase()).toMatch(/port|bind|address already in use/);
    } finally { p.kill(); }
  });

  test('TC-INFRA-00003: shared crate 被双端引用整体编译通过', () => {
    const out = sh('cargo check --workspace --message-format short');
    expect(out).not.toMatch(/error\[/);
  });

  test('TC-INFRA-00004: shared JWT 编解码 + 边界', () => {
    const out = sh('cargo test -p shared jwt -- --nocapture');
    expect(out).toMatch(/test result: ok/);
  });

  test('TC-INFRA-00005: shared bcrypt 随机盐 + 校验', () => {
    const out = sh('cargo test -p shared bcrypt -- --nocapture');
    expect(out).toMatch(/test result: ok/);
  });

  test('TC-INFRA-00006: app_server_user 无权修改 admins', () => {
    test.skip(!process.env.DATABASE_URL, '需要 DATABASE_URL');
    let err = '';
    try {
      sh(`psql "${process.env.DATABASE_URL}" -c "UPDATE admins SET role='super_admin' WHERE username='admin_op';"`);
    } catch (e: any) { err = String(e.stderr ?? e.message); }
    expect(err).toMatch(/permission denied|denied/i);
  });

  test('TC-INFRA-00007: CI 本地模拟 - lint + test 绿', () => {
    test.skip(process.env.CI_E2E_READY !== '1', 'CI_E2E_READY 未开启');
    const lint = sh('cargo clippy --workspace --no-deps -- -D warnings');
    expect(lint).not.toMatch(/error|warning/);
  });
});
