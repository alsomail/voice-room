/**
 * 测试套件：E2E GIFT 跨端打赏闭环
 * 用例来源：doc/tests/cases/E2E/TC-GIFT.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice + PlaywrightAgent）。
 *   已废弃：runMaestro()
 * 场景：Android U1 → 麦位 U2 送礼，WS 推送 + DB 事务 + Web Dashboard 统计
 */
import { test, expect } from '../support/fixtures';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { resetAndroidToLoginPage } from '../support/androidReset';

test.setTimeout(300_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

test.describe('TC-GIFT E2E - 跨端打赏', () => {
  test('TC-GIFT-00001: Android U1 向麦位 U2 送礼 多端闭环', async ({ e2eEnv, page }: any) => {
    const ANDROID_APP_ID = e2eEnv.androidAppId as string;
    if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
    const DATABASE_URL = e2eEnv.databaseUrl as string;
    if (!DATABASE_URL) throw new Error('e2eEnv.databaseUrl 未配置');
    const A = e2eEnv.userAId as string;
    const B = e2eEnv.userBId as string;
    const ROOM = e2eEnv.roomId as string;
    const APP_SERVER_BASE_URL = e2eEnv.appServerBaseUrl as string;

    if (!A || !B || !ROOM) {
      test.skip();
      return;
    }

    const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
    const phone = '+966500000900';

    // Step 0: 记录初值（DB 副作用基线）
    const a0 = Number(psql(DATABASE_URL, `SELECT coin_balance FROM users WHERE id='${A}'`));
    const b0 = Number(psql(DATABASE_URL, `SELECT coin_balance FROM users WHERE id='${B}'`));

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
      aiActionContext: '当前是 Android 语聊房 App，在房间内，底部有礼物按钮，点击弹出礼物面板',
    });

    try {
      // 前置：确保 U1 已登录并进入 ROOM（Round 3：标准化重置，不 pm clear）
      await resetAndroidToLoginPage(adbPrefix, ANDROID_APP_ID, 5, true);
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
      await agent.aiWaitFor('主界面可见', { timeoutMs: 20_000 });

      // 进入大厅，找到 E2E Test Room 并点击进入
      await agent.aiTap('大厅房间列表中的第一张房间卡片 或 文字含"E2E"的卡片');
      await agent.aiWaitFor('进入房间，底部操作栏可见', { timeoutMs: 15_000 });

      // Step 1-3: Android 打开礼物面板并送出礼物（火箭，价格 500 💎）
      await agent.aiTap('底部操作栏中的礼物按钮（🎁 图标）');
      await agent.aiWaitFor('礼物面板 Bottom Sheet 已弹出', { timeoutMs: 10_000 });
      await agent.aiTap('"火箭" 或 "Rocket" 礼物图标（价格 500 💎）');
      await agent.aiWaitFor('礼物选中（边框高亮）', { timeoutMs: 5_000 });
      await agent.aiTap('"送出" 按钮');
      await agent.aiWaitFor('礼物发送动画开始播放', { timeoutMs: 10_000 });
      await agent.aiAssert('L3 礼物特效动画已播放（全屏或大尺寸特效），礼物发送成功');

      // Step 4: DB 事务副作用断言（铁律 6）
      await new Promise(r => setTimeout(r, 3000)); // 等待 DB 写入
      const a1 = Number(psql(DATABASE_URL, `SELECT coin_balance FROM users WHERE id='${A}'`));
      const b1 = Number(psql(DATABASE_URL, `SELECT coin_balance FROM users WHERE id='${B}'`));
      expect(a0 - a1).toBe(500); // U1 扣除 500 💎
      expect(b1 - b0).toBeGreaterThan(0); // U2 获得收益（可能有平台抽成）
      const txCount = Number(psql(DATABASE_URL,
        `SELECT COUNT(*) FROM transactions WHERE user_id='${A}' AND delta=-500 AND created_at > NOW() - INTERVAL '5 minutes'`
      ));
      expect(txCount).toBeGreaterThanOrEqual(1);

    } finally {
      try { redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]); } catch { /* 忽略 */ }
      await agent.destroy().catch(() => {});
    }

    // Step 5: Web Dashboard 增量断言（Midscene Web）
    if (APP_SERVER_BASE_URL) {
      await page.goto(`${APP_SERVER_BASE_URL}/login`);
      const webAgent = new PlaywrightAgent(page);
      await webAgent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
      await page.waitForURL(/dashboard/, { timeout: 15_000 });
      await page.reload();
      await webAgent.aiAssert('"今日打赏总额"卡片的数字不小于 500（已包含本次送礼）');
    }
  });
});
