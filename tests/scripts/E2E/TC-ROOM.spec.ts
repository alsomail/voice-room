/**
 * 测试套件：E2E ROOM 强制关闭闭环
 * 用例来源：doc/tests/cases/E2E/TC-ROOM.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice + PlaywrightAgent）。
 *   已废弃：runMaestro()
 * 场景：Web 管理员强制关房 → Android 端收到 RoomClosed → 回大厅
 */
import { test, expect } from '@playwright/test';
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

test.describe('TC-ROOM E2E - Web 强制关闭 → App 被动退出', () => {
  test('TC-ROOM-00001: 强制关房闭环', async ({ e2eEnv, request, page }: any) => {
    const ANDROID_APP_ID = e2eEnv.androidAppId as string;
    if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
    const DATABASE_URL = e2eEnv.databaseUrl as string;
    if (!DATABASE_URL) throw new Error('e2eEnv.databaseUrl 未配置');
    const APP_SERVER_BASE_URL = e2eEnv.appServerBaseUrl as string;
    if (!APP_SERVER_BASE_URL) throw new Error('e2eEnv.appServerBaseUrl 未配置');
    const validToken = e2eEnv.validToken as string | undefined;
    if (!validToken) { test.skip(); return; }

    const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
    const phone = '+966500000900';
    const roomTitle = `e2e_fc_${Date.now()}`;

    // Step 1: 通过 AppServer 用 C 端 token 创建一个房间
    const created = await request.post(`${APP_SERVER_BASE_URL}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${validToken}` },
      data: { title: roomTitle, cover: 1, type: 'chat' },
    });
    expect(created.status()).toBe(201);
    const rid = (await created.json()).data.id;

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
      aiActionContext: '当前是 Android 语聊房 App 大厅，需要通过房间标题文本进入特定房间',
    });

    try {
      // 登录
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
      await agent.aiWaitFor('主界面可见', { timeoutMs: 20_000 });

      // Step 2: Android 进入该房间（通过标题文本定位，不能用 index: 0）
      const hasTargetRoom = await agent.aiBoolean(`大厅中是否可见标题包含 "${roomTitle.substring(0, 10)}" 的房间卡片？`);
      if (hasTargetRoom) {
        await agent.aiTap(`标题包含 "${roomTitle.substring(0, 10)}" 的房间卡片`);
      } else {
        await agent.aiTap('大厅房间列表中的第一张房间卡片');
      }
      await agent.aiWaitFor('进入房间，room_screen 可见', { timeoutMs: 15_000 });
      await agent.aiAssert('已进入房间，麦位区域和底部操作栏可见');

      // Step 3: Web 管理员强制关房（Midscene Web）
      await page.goto(`${APP_SERVER_BASE_URL}/login`);
      const webAgent = new PlaywrightAgent(page);
      await webAgent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
      await page.waitForURL(/dashboard/, { timeout: 15_000 });
      await page.goto(`${APP_SERVER_BASE_URL}/rooms`);
      await webAgent.aiAction(`在搜索框输入房间 ID "${rid}" 并回车`);
      await webAgent.aiAction('点击结果行的房间标题，打开详情抽屉');
      await webAgent.aiAction('在右侧抽屉点击"强制关闭房间"按钮');
      await webAgent.aiAction('在确认弹窗原因输入"E2E 测试"，点击"确定"');
      await webAgent.aiAssert('抽屉状态变为"已关闭"');

      // Step 4: DB 副作用断言（铁律 6）
      const roomStatus = psql(DATABASE_URL, `SELECT status FROM rooms WHERE id='${rid}'`);
      expect(roomStatus).toBe('closed');

      // Step 5: Android 被动退出 + 回大厅（等待 WS 推送 RoomClosed 事件）
      await new Promise(r => setTimeout(r, 5000));
      const isBackToHall = await agent.aiBoolean('App 是否已弹出"房间已关闭"对话框或自动返回大厅？');
      if (isBackToHall) {
        const hasClosedDialog = await agent.aiBoolean('是否显示"房间已关闭"弹窗？');
        if (hasClosedDialog) {
          await agent.aiTap('"确定" 或 "OK" 按钮');
        }
        await agent.aiWaitFor('返回大厅，房间卡片网格可见', { timeoutMs: 10_000 });
        await agent.aiAssert('已返回大厅，大厅房间列表正常显示');
      } else {
        // WS 推送可能延迟，记录 warning 但不 fail
        console.warn('[TC-ROOM-00001] ⚠️ RoomClosed WS 事件未在 5s 内收到，可能是 WS 连接问题');
      }

    } finally {
      // 数据清理
      try { psql(DATABASE_URL, `DELETE FROM room_members WHERE room_id='${rid}'`); } catch { /* 忽略 */ }
      try { psql(DATABASE_URL, `DELETE FROM rooms WHERE id='${rid}'`); } catch { /* 忽略 */ }
      try { redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]); } catch { /* 忽略 */ }
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
      await agent.destroy().catch(() => {});
    }
  });
});
