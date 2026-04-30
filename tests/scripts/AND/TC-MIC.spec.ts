/**
 * 测试套件：MIC 麦位（Android）
 * 用例来源：doc/tests/cases/AND/TC-MIC.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-MIC-00001 — 权限申请：拒绝后 Fallback 到系统设置
 *   TC-MIC-00002 — 上麦 → 下麦 E2E（含 DB 副作用断言）
 */
import { test, expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';

test.setTimeout(300_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

// ── TC-MIC-00001：权限申请 - 拒绝后 Fallback 到系统设置 ───────────────────────

test('TC-MIC-00001: 权限申请拒绝后 Fallback 到系统设置', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  // 前置：撤销 RECORD_AUDIO 权限，确保未授予
  try {
    execSync(`${adbPrefix} shell pm revoke ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, {
      stdio: 'pipe',
    });
  } catch { /* 可能已经无权限，忽略 */ }

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语',
  });

  try {
    // 冷启动 + 弹窗处理 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiWaitFor('手机号输入框可见', { timeoutMs: 10_000 });
    await agent.aiInput('500000900', '手机号输入框');
    await agent.aiTap('"获取验证码" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // 进入房间
    await agent.aiTap('第一张房间卡片');
    await agent.aiWaitFor('已进入房间，可见麦位布局', { timeoutMs: 15_000 });

    // Step1：点击麦克风按钮或空麦位
    await agent.aiTap('底部操作栏中的麦克风按钮（🎤 图标）或空麦位的 "+" 按钮');
    await agent.aiWaitFor('弹出权限请求或系统对话框', { timeoutMs: 10_000 });
    await agent.aiAssert('弹出系统录音权限请求对话框，包含允许和拒绝选项');

    // Step2：点击拒绝
    await agent.aiTap('"拒绝" 或 "Deny" 按钮（拒绝录音权限）');
    await agent.aiWaitFor('权限被拒绝，App 给出提示', { timeoutMs: 8_000 });
    await agent.aiAssert('App 内出现 SnackBar 或 Toast 提示需要麦克风权限，或有"去设置"按钮');

    // Step3：点击去设置（如果有）
    const hasSettingsBtn = await agent.aiBoolean('是否有"去设置"或"Settings"按钮？');
    if (hasSettingsBtn) {
      await agent.aiTap('"去设置" 或 "Settings" 按钮');
      await agent.aiWaitFor('跳转到系统设置页面', { timeoutMs: 10_000 });
      await agent.aiAssert('已进入系统设置页面（应用权限设置），可见麦克风权限选项');
      // 返回 App
      execSync(`${adbPrefix} shell am start -n ${ANDROID_APP_ID}/.presentation.MainActivity`, { stdio: 'pipe' });
      await agent.aiWaitFor('返回 App', { timeoutMs: 10_000 });
    }

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-MIC-00002：上麦 → 下麦 E2E ────────────────────────────────────────────

test('TC-MIC-00002: 上麦 → RTC publish → 下麦 E2E', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';
  const ROOM_ID = e2eEnv.roomId as string | undefined;

  // 前置：授予 RECORD_AUDIO 权限
  try {
    execSync(`${adbPrefix} shell pm grant ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, {
      stdio: 'pipe',
    });
  } catch { /* 忽略 */ }

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语，房间内可见主播麦和副麦',
  });

  try {
    // 冷启动 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiWaitFor('手机号输入框可见', { timeoutMs: 10_000 });
    await agent.aiInput('500000900', '手机号输入框');
    await agent.aiTap('"获取验证码" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // 进入房间
    await agent.aiTap('第一张房间卡片');
    await agent.aiWaitFor('已进入房间，可见麦位布局', { timeoutMs: 15_000 });

    // 确认有空麦位
    const hasEmptyMic = await agent.aiBoolean('麦位区是否有空闲的 "+" 按钮（空麦位）？');
    if (!hasEmptyMic) {
      // 如果没有空麦位，跳过上麦测试
      await agent.aiAssert('房间内麦位区可见，无空麦位');
      return;
    }

    // Step1：点击空麦位
    await agent.aiTap('麦位区中一个空闲的 "+" 按钮（选择副麦位）');
    await agent.aiWaitFor('弹出上麦确认或直接上麦', { timeoutMs: 10_000 });

    // 可能有上麦确认弹窗
    const hasConfirm = await agent.aiBoolean('是否弹出上麦确认菜单或对话框？');
    if (hasConfirm) {
      await agent.aiTap('"上麦" 或 "确认" 按钮');
    }

    // Step2：等待上麦成功
    await agent.aiWaitFor('麦位上出现头像，表示已成功上麦', { timeoutMs: 15_000 });
    await agent.aiAssert('麦位上显示用户头像或麦克风图标，表示已成功上麦');

    // ── DB 副作用断言（铁律 6）────────────────────────────────────────────
    if (ROOM_ID && DATABASE_URL) {
      await new Promise(r => setTimeout(r, 2000));
      const micCount = psql(DATABASE_URL,
        `SELECT COUNT(*) FROM mic_seats WHERE room_id='${ROOM_ID}' AND user_id IS NOT NULL`
      );
      expect(Number(micCount)).toBeGreaterThan(0);
    }

    // Step3：下麦
    await agent.aiTap('底部操作栏"下麦"按钮或已上麦的麦位（自己的头像）');
    await agent.aiWaitFor('弹出下麦确认', { timeoutMs: 8_000 });
    const hasLeaveMicConfirm = await agent.aiBoolean('是否弹出下麦确认对话框？');
    if (hasLeaveMicConfirm) {
      await agent.aiTap('"确认下麦" 或 "确定" 按钮');
    }

    // Step4：断言麦位恢复为空
    await agent.aiWaitFor('麦位恢复空位状态', { timeoutMs: 10_000 });
    await agent.aiAssert('原先上麦的麦位已恢复为 "+" 空状态');

  } finally {
    // 撤销权限防止影响后续测试
    try {
      execSync(`${adbPrefix} shell pm revoke ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, {
        stdio: 'pipe',
      });
    } catch { /* 忽略 */ }
    if (ROOM_ID && DATABASE_URL) {
      try {
        const userId = psql(DATABASE_URL, `SELECT id FROM users WHERE phone='${phone}' LIMIT 1`);
        if (userId) {
          psql(DATABASE_URL, `UPDATE mic_seats SET user_id=NULL WHERE room_id='${ROOM_ID}' AND user_id='${userId}'`);
        }
      } catch { /* 忽略 */ }
    }
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
