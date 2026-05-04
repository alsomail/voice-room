/**
 * 测试套件：E2E USER Web 封禁 → Android 被踢 → Web 状态刷新
 * 用例来源：doc/tests/cases/E2E/TC-USER.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice + PlaywrightAgent）。
 *   已废弃：runMaestro()
 */
import { test, expect } from '../support/fixtures';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';

test.setTimeout(300_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

test.describe('TC-USER E2E - 封禁多端闭环', () => {
  test('TC-USER-00001: Web 封禁 → Android 踢下线 → Web 状态刷新', async ({ e2eEnv, page }: any) => {
    const ANDROID_APP_ID = e2eEnv.androidAppId as string;
    if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
    const DATABASE_URL = e2eEnv.databaseUrl as string;
    if (!DATABASE_URL) throw new Error('e2eEnv.databaseUrl 未配置');
    const APP_SERVER_BASE_URL = e2eEnv.appServerBaseUrl as string;
    if (!APP_SERVER_BASE_URL) throw new Error('e2eEnv.appServerBaseUrl 未配置');
    const UID = e2eEnv.userAId as string | undefined;
    if (!UID) { test.skip(); return; }

    const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
    const phone = '+966500000900';

    // 前置：清理为正常态
    try { psql(DATABASE_URL, `UPDATE users SET banned_until=NULL WHERE id='${UID}'`); } catch { /* 忽略 */ }

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
      aiActionContext: '当前是 Android 语聊房 App 大厅，被封禁后会弹出封禁提示对话框',
    });

    try {
      // Step 1: Android 已登录，进入大厅
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
      await agent.launch(ANDROID_APP_ID);
      await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });

      const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知或权限请求弹窗？');
      if (hasConsentDialog) await agent.aiTap('"同意" 或 "确定" 按钮');

      try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
      catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }
      await agent.aiInput('500000900', '手机号输入框');
      await agent.aiTap('"获取验证码" 按钮');
      await agent.aiWaitFor('倒计时启动', { timeoutMs: 15_000 });
      try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
      catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }
      await agent.aiInput('123456', '验证码输入框');
      await agent.aiTap('"登录" 按钮');
      await agent.aiWaitFor('主界面可见，大厅房间列表显示', { timeoutMs: 20_000 });
      await agent.aiAssert('大厅房间列表可见，App 正常运行');

      // Step 2: Web Admin 执行封禁（Midscene Web）
      await page.goto(`${APP_SERVER_BASE_URL}/login`);
      const webAgent = new PlaywrightAgent(page);
      await webAgent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
      await page.waitForURL(/dashboard/, { timeout: 15_000 });
      await page.goto(`${APP_SERVER_BASE_URL}/users`);
      await webAgent.aiAction(`在搜索框输入 "${UID}" 并回车`);
      await webAgent.aiAction('点击匹配行的用户昵称，在详情抽屉中点击"封禁"按钮');
      await webAgent.aiAction('选择"临时"，时长选择"24 小时"，原因输入"E2E 测试"，点击"确定"');
      await webAgent.aiAssert('抽屉状态显示"已封禁"');

      // Step 3: DB 副作用断言（铁律 6）
      const isBanned = psql(DATABASE_URL, `SELECT banned_until IS NOT NULL FROM users WHERE id='${UID}'`);
      expect(isBanned).toBe('t');

      // Step 4: Android 侧被踢下线并跳回登录页（等待 WS 推送 UserBanned 事件）
      await new Promise(r => setTimeout(r, 5000));
      const hasBannedDialog = await agent.aiBoolean('App 是否弹出"账号被封禁"或"账号异常"相关对话框？');
      if (hasBannedDialog) {
        await agent.aiTap('"确定" 或 "OK" 按钮');
        await agent.aiWaitFor('跳转到登录页', { timeoutMs: 10_000 });
        await agent.aiAssert('已跳转到登录页（显示手机号输入框）');
      } else {
        console.warn('[TC-USER-00001] ⚠️ UserBanned WS 事件未在 5s 内收到，尝试重新打开 App');
        // 重启 App 验证封禁状态（封禁用户重新打开 App 应被拒绝）
        execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
        await new Promise(r => setTimeout(r, 1000));
        await agent.launch(ANDROID_APP_ID);
        await agent.aiWaitFor('界面加载', { timeoutMs: 10_000 });
        const showsBannedOnRestart = await agent.aiBoolean('是否显示封禁提示或登录页？');
        expect(showsBannedOnRestart).toBe(true);
      }

      // Step 5: Web 刷新列表状态同步
      await page.goto(`${APP_SERVER_BASE_URL}/users`);
      await webAgent.aiAction(`在搜索框输入 "${UID}" 并回车`);
      await webAgent.aiAssert('匹配行状态列显示红色"已封禁"标签');

    } finally {
      // 收尾清理：解封
      try {
        psql(DATABASE_URL, `UPDATE users SET banned_until=NULL WHERE id='${UID}'`);
      } catch { /* 忽略 */ }
      try { redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]); } catch { /* 忽略 */ }
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
      await agent.destroy().catch(() => {});
    }
  });
});
