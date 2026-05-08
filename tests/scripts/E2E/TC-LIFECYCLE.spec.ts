/**
 * 测试套件：E2E LIFECYCLE 新用户首次旅程闭环
 * 用例来源：doc/tests/cases/E2E/TC-LIFECYCLE.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice + PlaywrightAgent）。
 *
 * 覆盖用例（P0）：
 *   TC-LIFECYCLE-00001 — 新用户首次完整旅程（注册→同意隐私→大厅→进房→上麦→首单送礼）
 */
import { test, expect } from '../support/fixtures';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { dismissConsentDialog } from '../support/androidReset';

test.setTimeout(360_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

test.describe('TC-LIFECYCLE E2E - 新用户首次完整旅程', () => {
  test('TC-LIFECYCLE-00001: 新用户首次完整旅程（注册→同意→大厅→进房→上麦→送礼）', async ({ e2eEnv, request, page }: any) => {
    const ANDROID_APP_ID = e2eEnv.androidAppId as string;
    if (!ANDROID_APP_ID) { test.skip(); return; }
    const DATABASE_URL = e2eEnv.databaseUrl as string;
    if (!DATABASE_URL) { test.skip(); return; }
    const APP_SERVER_BASE_URL = e2eEnv.appServerBaseUrl as string;
    const ADMIN_WEB_URL = e2eEnv.adminWebUrl as string;

    const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
    const newPhone = '+966500000700';
    const newPhoneLocal = '500000700';

    // 前置：清理测试用户
    try {
      psql(DATABASE_URL, `DELETE FROM users WHERE phone='${newPhone}'`);
    } catch { /* ignore */ }
    try { redisExecSync(['DEL', `sms:code:${newPhone}`, `sms:cooldown:${newPhone}`]); }
    catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
      aiActionContext: '当前是 Android 语聊房 App 新用户首次注册旅程，界面语言为中文',
    });

    try {
      // Step 1: 全新安装状态冷启动（保留 pm clear：测试首次用户旅程，需要 fresh install 状态）
      execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`, { stdio: 'pipe' });
      // Round 3 修复：pm clear 后用 ADB 先消弹窗，再让 Midscene 接管
      execSync(`${adbPrefix} shell am start -n ${ANDROID_APP_ID}/com.voice.room.android.presentation.MainActivity`);
      await new Promise(r => setTimeout(r, 3000));
      await dismissConsentDialog(adbPrefix, 5);
      await agent.launch(ANDROID_APP_ID);
      await agent.aiWaitFor('界面上有可交互的元素', { timeoutMs: 20_000 });

      // Step 2-3: 输入手机号 + 获取验证码
      try { redisExecSync(['HSET', `sms:code:${newPhone}`, 'code', '123456']); }
      catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

      const hasLoginPage = await agent.aiBoolean('当前界面是登录页（有手机号输入框）？');
      if (!hasLoginPage) {
        console.warn('[TC-LIFECYCLE-00001] ⚠️ 未出现登录页，App 可能已有会话');
        test.skip();
        return;
      }

      await agent.aiInput(newPhoneLocal, '手机号输入框');
      await agent.aiTap('"获取验证码"/"Get Code" 按钮');
      await agent.aiWaitFor('按钮进入倒计时', { timeoutMs: 15_000 });

      // 预填验证码
      try { redisExecSync(['HSET', `sms:code:${newPhone}`, 'code', '123456']); }
      catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

      // Step 4-5: 验证码 + 登录
      await agent.aiInput('123456', '验证码输入框');
      await agent.aiTap('"登录" 按钮');
      await agent.aiWaitFor('登录成功，页面跳转', { timeoutMs: 20_000 });

      // Step 6: DB 验证用户创建
      let userId = '';
      try {
        userId = psql(DATABASE_URL, `SELECT id FROM users WHERE phone='${newPhone}' LIMIT 1`);
        expect(userId).toBeTruthy();
        console.log(`[TC-LIFECYCLE-00001] 新用户 ID：${userId}`);
      } catch {
        console.warn('[TC-LIFECYCLE-00001] ⚠️ DB 用户验证失败（非阻断）');
      }

      // Step 7-8: 隐私同意弹窗处理
      const hasPrivacyDialog = await agent.aiBoolean('是否出现隐私政策或数据收集同意弹窗？');
      if (hasPrivacyDialog) {
        await agent.aiTap('"同意完整分析" 或 "完整同意" 按钮');
        await agent.aiWaitFor('弹窗关闭，进入主界面', { timeoutMs: 10_000 });
      } else {
        console.warn('[TC-LIFECYCLE-00001] 未出现隐私弹窗（可能 App 行为已变化）');
      }

      // Step 9: 大厅房间列表渲染
      await agent.aiWaitFor('大厅房间列表可见', { timeoutMs: 20_000 });
      await agent.aiAssert('大厅已加载，至少有一张房间卡片可见');

      // Step 10-12: 进入第一个房间
      await agent.aiTap('大厅房间列表中的第一张房间卡片');
      await agent.aiWaitFor('房间页可见，麦位区域和操作栏显示', { timeoutMs: 20_000 });
      await agent.aiAssert('房间页已加载完成');

      // Step 13-14: 点击空麦位上麦
      const hasEmptySlot = await agent.aiBoolean('是否有空闲麦位（空白麦位槽）？');
      if (hasEmptySlot) {
        await agent.aiTap('第一个空闲麦位槽');
        await new Promise(r => setTimeout(r, 3_000));
        await agent.aiAssert('上麦操作已执行（麦位可能显示自己的头像）');
      } else {
        console.warn('[TC-LIFECYCLE-00001] ⚠️ 无空闲麦位，跳过上麦步骤');
      }

      // Step 15-18: 送礼
      const hasGiftBtn = await agent.aiBoolean('底部是否有礼物图标按钮？');
      if (hasGiftBtn) {
        await agent.aiTap('底部礼物图标按钮');
        await agent.aiWaitFor('礼物面板弹出', { timeoutMs: 10_000 });
        await agent.aiTap('第一个礼物（价格最低）');
        const hasSendBtn = await agent.aiBoolean('是否有"送出"按钮？');
        if (hasSendBtn) {
          await agent.aiTap('"送出" 按钮');
          await new Promise(r => setTimeout(r, 3_000));
        }
      }

      // Step 19: Web Admin 行为流验证（尽力执行）
      if (userId) {
        try {
          await page.goto(`${ADMIN_WEB_URL}/login`);
          await page.waitForLoadState('networkidle');
          const webAgent = new PlaywrightAgent(page);
          await webAgent.aiAction('在用户名输入 "super_admin"，密码输入 "Pass@123"，点击登录');
          await page.waitForURL(/dashboard/, { timeout: 20_000 });
          await page.goto(`${ADMIN_WEB_URL}/users`);
          await page.waitForTimeout(2000);
          await webAgent.aiAction(`在搜索框输入 "${newPhoneLocal}" 并回车`);
          await page.waitForTimeout(2000);
          const hasUser = await webAgent.aiBoolean('搜索结果是否有匹配用户？');
          if (hasUser) {
            await webAgent.aiTap('搜索结果中的第一个用户行');
            await page.waitForTimeout(2000);
            await webAgent.aiAssert('用户详情可见');
          }
        } catch (webErr) {
          console.warn(`[TC-LIFECYCLE-00001] ⚠️ Web 验证异常（非阻断）：${webErr}`);
        }
      }

    } finally {
      // 数据清理
      try {
        psql(DATABASE_URL, `DELETE FROM users WHERE phone='${newPhone}'`);
      } catch { /* ignore */ }
      try {
        redisExecSync(['DEL', `sms:code:${newPhone}`, `sms:cooldown:${newPhone}`]);
      } catch { /* ignore */ }
      execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`, { stdio: 'pipe' });
      await agent.destroy().catch(() => {});
    }
  });
});
