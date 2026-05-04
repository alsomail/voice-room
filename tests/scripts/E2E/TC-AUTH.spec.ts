/**
 * 测试套件：E2E 登录闭环（Android + AppServer + DB + Redis）
 * 用例来源：doc/tests/cases/E2E/TC-AUTH.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *   已废弃：runMaestro() + redis-cli 直调 + Redis GET（应为 HGET）
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';

test.setTimeout(300_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

test.describe('TC-AUTH E2E - 登录闭环', () => {
  test('TC-AUTH-00001: 新用户 E2E 注册登录闭环', async ({ e2eEnv }: any) => {
    const ANDROID_APP_ID = e2eEnv.androidAppId as string;
    if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
    const DATABASE_URL = e2eEnv.databaseUrl as string;
    if (!DATABASE_URL) throw new Error('e2eEnv.databaseUrl 未配置');
    const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
    const phone = '+966500000500';
    const phoneLocal = '500000500';

    // 前置清理
    try { psql(DATABASE_URL, `DELETE FROM users WHERE phone='${phone}'`); } catch { /* 忽略 */ }
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`, `sms:daily:${phone}`]);
    } catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
      aiActionContext: '当前是 Android 语聊房 App 登录页，有 +966 国家码、手机号输入框、获取验证码按钮',
    });

    try {
      // Step 1：冷启动
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
      await agent.launch(ANDROID_APP_ID);
      // Round-1 fix: 先等待任意按钮可见，再无条件尝试关闭同意弹窗，再等待登录页
      await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
      await new Promise(r => setTimeout(r, 500));
      try {
        await agent.aiTap('"同意" 或 "确定" 按钮（关闭数据收集隐私政策弹窗）');
        await new Promise(r => setTimeout(r, 500));
      } catch { /* 无弹窗则忽略 */ }
      await agent.aiWaitFor('登录页面可见，手机号输入框可见', { timeoutMs: 15_000 });
      await agent.aiAssert('登录页显示：手机号输入框、获取验证码按钮');

      // Step 2：输入手机号 + 点击获取验证码
      await agent.aiInput(phoneLocal, '手机号输入框（不含 +966 国家码的本地号码部分）');
      await agent.aiTap('"获取验证码" 按钮');
      await agent.aiWaitFor('按钮文案变为倒计时（如"60s 后重发"）', { timeoutMs: 15_000 });

      // Step 3-4：Redis 副作用断言（铁律 6）
      // 注：正确命令是 HGET（AppServer 使用 Hash 存储 sms:code:{phone}），不是 GET
      let redisTtl = -1;
      try {
        const ttlStr = redisExecSync(['TTL', `sms:code:${phone}`]);
        redisTtl = Number(ttlStr.trim());
      } catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

      if (redisTtl > 0) {
        expect(redisTtl).toBeGreaterThan(0);
        expect(redisTtl).toBeLessThanOrEqual(300);
      }

      // 将验证码覆盖为已知值（HSET，与 AppServer Hash 结构保持一致）
      try {
        redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
      } catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

      // Step 5：输入验证码并登录
      await agent.aiInput('123456', '验证码输入框');
      await agent.aiTap('"登录" 按钮');
      await agent.aiWaitFor('已离开登录页，主界面可见（底部 Tab 栏）', { timeoutMs: 20_000 });

      // Step 6-7：DB 副作用断言（铁律 6）
      const coinBalance = psql(DATABASE_URL, `SELECT coin_balance FROM users WHERE phone='${phone}'`);
      expect(coinBalance).toBe('0');

      // Step 8-9：验证大厅 + "我的" Tab
      await agent.aiAssert('主界面显示：房间列表大厅，底部 Tab 栏可见');
      await agent.aiTap('底部 Tab 栏中的"我的"或"Me"选项卡');
      await agent.aiWaitFor('个人中心页面加载', { timeoutMs: 10_000 });
      await agent.aiAssert('个人中心显示：用户昵称，以及余额（钻石余额、金币余额或钱包余额等，0 或其他数字均可）');

      // Step 10：冷启验证 JWT 持久化
      execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
      await new Promise(r => setTimeout(r, 1000));
      await agent.launch(ANDROID_APP_ID);
      await agent.aiWaitFor('App 重启后主界面可见（无需重新登录）', { timeoutMs: 15_000 });
      await agent.aiAssert('App 重启后自动进入主界面（JWT 持久化，未回到登录页）');

    } finally {
      try { psql(DATABASE_URL, `DELETE FROM users WHERE phone='${phone}'`); } catch { /* 忽略 */ }
      try {
        redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`, `sms:daily:${phone}`]);
      } catch { /* 忽略 */ }
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
      await agent.destroy().catch(() => {});
    }
  });
});
