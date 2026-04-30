/**
 * TC-AUTH-00003：Android 端注册登录（Midscene 视觉驱动）
 *
 * 用例来源：doc/tests/cases/AND/TC-AUTH.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖步骤：
 *   Step0  — pm clear 冷启动 + 首次同意弹窗处理
 *   Step1  — 输入手机号
 *   Step2  — 点击获取验证码，断言倒计时态
 *   Step3  — Redis HGET 读取验证码（副作用断言），覆写为已知值 123456
 *   Step4  — 输入验证码
 *   Step5  — 点击登录
 *   Step6  — DB 副作用：users 表有该手机号记录
 *   Step7  — 视觉断言：大厅三 Tab 可见（登录成功）
 *   Step8  — force-stop 后重启，断言未跳回登录页（JWT 持久化验证）
 */
import { test, expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';

test.setTimeout(300_000);

// ── 工具函数 ────────────────────────────────────────────────────────────────

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

const redis = (args: string[]): string => redisExecSync(args);

// ── 用例 ─────────────────────────────────────────────────────────────────────

test('TC-AUTH-00003: Android 端注册登录全链路', async ({ e2eEnv }: any) => {
  // ✅ 从 fixture 读取运行时参数，禁止在 test() 外部用 process.env
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置 — 请在 tests/scripts/env/.env.local 中设置 ANDROID_APP_ID');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

  const phone = '+966500000100';
  const phoneLocal = '500000100';

  // ── 前置清理（FK 顺序：先删 rooms 再删 users）─────────────────────────────
  psql(DATABASE_URL, `DELETE FROM rooms WHERE owner_id = (SELECT id FROM users WHERE phone='${phone}' LIMIT 1)`);
  psql(DATABASE_URL, `DELETE FROM users WHERE phone='${phone}'`);
  try {
    redis(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`, `sms:daily:${phone}`]);
  } catch (e) {
    if (!(e instanceof RedisCliUnavailableError)) throw e;
  }

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语',
  });

  try {
    // ── Step0：冷启动 + 首次同意弹窗处理 ─────────────────────────────────────
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    await agent.launch(ANDROID_APP_ID);
    // 等第一个可交互元素（弹窗或登录页均可）
    await agent.aiWaitFor('界面上有可交互的按钮或输入框（弹窗或登录页均可）', { timeoutMs: 15_000 });
    // 探测并关闭首次同意弹窗
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    await agent.aiWaitFor('手机号输入框可见，登录页面已加载完成', { timeoutMs: 10_000 });

    // ── Step1：输入手机号 ────────────────────────────────────────────────────
    await agent.aiInput(phoneLocal, '手机号输入框');

    // ── Step2：发送验证码 ────────────────────────────────────────────────────
    await agent.aiTap('"获取验证码" 按钮');
    await agent.aiAssert('按钮文案变为 "60s 后重发" 或类似倒计时态');

    // ── Step3：Redis 副作用断言（SMS 验证码为 Hash 结构，用 HGET）──────────
    const ttl = Number(redis(['TTL', `sms:code:${phone}`]));
    expect(ttl).toBeGreaterThan(0);
    const code = redis(['HGET', `sms:code:${phone}`, 'code']);
    expect(code).toMatch(/^\d{6}$/);
    // 覆写为已知值，确保后续输入可控
    redis(['HSET', `sms:code:${phone}`, 'code', '123456']);

    // ── Step4：输入验证码 ────────────────────────────────────────────────────
    await agent.aiInput('123456', '验证码输入框');

    // ── Step5：点击登录 ──────────────────────────────────────────────────────
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('登录请求已发出，页面正在跳转', { timeoutMs: 15_000 });

    // ── Step6：DB 副作用断言 ─────────────────────────────────────────────────
    const count = psql(DATABASE_URL, `SELECT COUNT(*) FROM users WHERE phone='${phone}'`);
    expect(count).toBe('1');

    // ── Step7：视觉断言 — 大厅三 Tab 可见 ──────────────────────────────────
    await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });
    await agent.aiAssert('底部有三个 Tab（如：首页、发现、我的），或房间大厅列表可见');

    // ── Step8：JWT 持久化断言 ────────────────────────────────────────────────
    // force-stop 后重启，期望不跳回登录页（token 已持久化）
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
    await new Promise(r => setTimeout(r, 1500));
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面加载完成', { timeoutMs: 20_000 });
    // 处理可能再次出现的弹窗（force-stop 不清数据，通常不会，但保险处理）
    const hasDialog2 = await agent.aiBoolean('当前界面是否存在弹窗？');
    if (hasDialog2) {
      await agent.aiTap('"同意" 或 "确定" 或 "关闭" 按钮');
    }
    await agent.aiAssert('当前页面是大厅或房间列表，不是手机号登录输入页', {
      errorMessage: 'JWT 持久化失败：force-stop 后重启仍显示登录页，token 未落盘',
    });
  } finally {
    // ── 清理（FK 顺序）──────────────────────────────────────────────────────
    try {
      psql(DATABASE_URL, `DELETE FROM rooms WHERE owner_id = (SELECT id FROM users WHERE phone='${phone}' LIMIT 1)`);
      psql(DATABASE_URL, `DELETE FROM users WHERE phone='${phone}'`);
    } catch { /* 清理失败不影响用例结果 */ }
    try {
      redis(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`, `sms:daily:${phone}`]);
    } catch { /* redis 不可用时忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
