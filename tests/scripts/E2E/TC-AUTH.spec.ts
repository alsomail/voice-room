/**
 * 测试套件：E2E 登录闭环（Android + AppServer + DB + Redis）
 * 用例来源：doc/tests/cases/E2E/TC-AUTH.md
 * 说明：Playwright 作为调度器，通过 execSync 驱动 Maestro 执行 Android 步骤，
 *       同时直接访问 DB/Redis/AppServer 完成跨端断言。
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';

const APP_BASE = process.env.APP_SERVER_BASE_URL!;

const redis = (cmd: string): string =>
  execSync(`redis-cli ${cmd}`, { encoding: 'utf-8' }).trim();

const psql = (sql: string): string =>
  execSync(
    `psql "${process.env.DATABASE_URL!}" -tA -c "${sql.replace(/"/g, '\\"')}"`,
    { encoding: 'utf-8' },
  ).trim();

function runMaestro(yaml: string) {
  const tmp = path.join(os.tmpdir(), `maestro_${Date.now()}.yaml`);
  fs.writeFileSync(tmp, yaml, 'utf-8');
  try {
    execSync(`maestro test "${tmp}"`, { stdio: 'inherit' });
  } finally {
    fs.unlinkSync(tmp);
  }
}

test.describe('TC-AUTH E2E - 登录闭环', () => {
  test('TC-AUTH-00001: 新用户 E2E 注册登录闭环', async ({ request }) => {
    const phone = '+966500000500';
    const phoneLocal = '500000500';
    // 前置清理
    psql(`DELETE FROM users WHERE phone='${phone}'`);
    redis(`DEL sms:code:${phone} sms:cooldown:${phone} sms:daily:${phone}`);

    // Step 1-2: Android 启动 App → 输入手机号 → 获取验证码
    runMaestro(`
appId: ${process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug'}
---
- launchApp:
    clearState: true
- tapOn:
    id: "phone_input"
- inputText: "${phoneLocal}"
- tapOn: "获取验证码"
- assertVisible: "s 后重发|Resend in"
`);

    // Step 3-4: 断言 AppServer 发码 + Redis code 存在
    const codeTtl = Number(redis(`TTL sms:code:${phone}`));
    expect(codeTtl).toBeGreaterThan(0);
    expect(codeTtl).toBeLessThanOrEqual(300);
    expect(redis(`GET sms:code:${phone}`)).toMatch(/^\d{6}$/);

    // 为确定性登录，将验证码固定为 123456（Mock SMS 模式下可直接覆盖）
    redis(`SET sms:code:${phone} 123456 EX 300`);

    // Step 5: Android 输入验证码并登录
    runMaestro(`
appId: ${process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug'}
---
- launchApp
- tapOn:
    id: "code_input"
- inputText: "123456"
- tapOn: "登录"
- extendedWaitUntil:
    visible: "语聊房|Voice Room"
    timeout: 8000
`);

    // Step 6-7: AppServer 日志 & DB 断言
    const row = psql(`SELECT coin_balance FROM users WHERE phone='${phone}'`);
    expect(row).toBe('0');

    // Step 8-9: 大厅显示 + “我的” Tab
    runMaestro(`
appId: ${process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug'}
---
- launchApp
- assertVisible: "语聊房|Voice Room"
- tapOn: "我的|Me"
- assertVisible: "User_"
- assertVisible: "0"
`);

    // Step 10: 冷启验证 JWT 持久化
    runMaestro(`
appId: ${process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug'}
---
- stopApp
- launchApp
- extendedWaitUntil:
    visible: "语聊房|Voice Room"
    timeout: 5000
- assertNotVisible: "登录|Login"
`);

    // 数据清理
    psql(`DELETE FROM users WHERE phone='${phone}'`);
    redis(`DEL sms:code:${phone} sms:cooldown:${phone} sms:daily:${phone}`);
  });
});
