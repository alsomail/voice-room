/**
 * 测试套件：Android 弹性场景（Resilience）
 * 用例来源：doc/tests/cases/AND/TC-RESILIENCE.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0 优先）：
 *   TC-RESILIENCE-00001 — WS 断线后自动指数退避重连 + 状态恢复
 *   TC-RESILIENCE-00004 — App 前后台切换（30s 内无重连 / 30s 后新 session）
 *   TC-RESILIENCE-00005 — 进程被杀后冷启动 JWT 持久化
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { resetAndroidToLoginPage, resetAndroidToMainPage } from '../support/androidReset';

test.setTimeout(300_000);

// ── 公共登录帮手 ─────────────────────────────────────────────────────────────

async function loginUser(agent: any, adbPrefix: string, phone: string, phoneLocal: string) {
  try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
  catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

  await agent.aiWaitFor('手机号输入框可见或主界面已加载', { timeoutMs: 15_000 });
  const alreadyLoggedIn = await agent.aiBoolean('当前界面是主界面大厅（有房间列表），而不是登录页？');
  if (alreadyLoggedIn) return;

  await agent.aiInput(phoneLocal, '手机号输入框');
  await agent.aiTap('"获取验证码"/"Get Code" 按钮');
  await agent.aiWaitFor('倒计时启动', { timeoutMs: 10_000 });
  try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
  catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }
  await agent.aiInput('123456', '验证码输入框');
  await agent.aiTap('登录 或 确认 按钮');
  await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });
}

// ── TC-RESILIENCE-00001：WS 断线自动重连 ──────────────────────────────────────

test('TC-RESILIENCE-00001: WS 断线后自动指数退避重连 + 状态恢复', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) test.skip();
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';
  const phoneLocal = '500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，正在测试 WebSocket 断线重连弹性场景',
  });

  try {
    // Round 5 修复（方案 D）：JWT 注入绕过 UI 登录流（agent.launch() 移除，避免 HOME 闪屏）
    await resetAndroidToMainPage(adbPrefix, ANDROID_APP_ID, phone);
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });
    await agent.aiTap('大厅房间列表中的第一张房间卡片');
    await agent.aiWaitFor('进入房间页，麦位区域和公屏可见', { timeoutMs: 20_000 });

    // Step 1: 断网 → 观察重连横条
    execSync(`${adbPrefix} shell svc wifi disable && ${adbPrefix} shell svc data disable`, { stdio: 'pipe' });
    await new Promise(r => setTimeout(r, 3_000));
    const hasDisconnectedBanner = await agent.aiBoolean('是否出现网络断开提示横条或重连提示？');
    expect(hasDisconnectedBanner).toBe(true);

    // Step 2: 等待 5s，验证重连尝试
    await new Promise(r => setTimeout(r, 5_000));
    const isRetrying = await agent.aiBoolean('是否可见"正在重连"或"重试中"类提示？');
    expect(isRetrying).toBe(true);

    // Step 3: 恢复网络 → 验证状态恢复
    execSync(`${adbPrefix} shell svc wifi enable && ${adbPrefix} shell svc data enable`, { stdio: 'pipe' });
    await new Promise(r => setTimeout(r, 5_000));
    const isRecovered = await agent.aiBoolean('网络连接已恢复，横条提示是否已消失或显示已连接？');
    expect(isRecovered).toBe(true);

  } finally {
    try { execSync(`${adbPrefix} shell svc wifi enable && ${adbPrefix} shell svc data enable`, { stdio: 'pipe' }); } catch { /* ignore */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-RESILIENCE-00004：前后台切换 ──────────────────────────────────────────

test('TC-RESILIENCE-00004: App 前后台切换 - 30s 内归来无重连', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) test.skip();
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';
  const phoneLocal = '500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，正在测试前后台切换场景',
  });

  try {
    // Round 5 修复（方案 D）：JWT 注入绕过 UI 登录流（agent.launch() 移除，避免 HOME 闪屏）
    await resetAndroidToMainPage(adbPrefix, ANDROID_APP_ID, phone);
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // 进入房间
    await agent.aiTap('大厅房间列表中的第一张房间卡片');
    await agent.aiWaitFor('房间页可见', { timeoutMs: 20_000 });

    // Step 1: 按 Home 回到桌面，等待 10s（30s 内，保持连接）
    execSync(`${adbPrefix} shell input keyevent KEYCODE_HOME`, { stdio: 'pipe' });
    await new Promise(r => setTimeout(r, 10_000));

    // Step 2: 切回 App → 无重连横条
    execSync(`${adbPrefix} shell am start -n ${ANDROID_APP_ID}/com.voice.room.android.presentation.MainActivity`, { stdio: 'pipe' });
    await new Promise(r => setTimeout(r, 3_000));
    await agent.aiWaitFor('App 已回到前台', { timeoutMs: 10_000 });
    const hasReconnectBanner = await agent.aiBoolean('是否出现"重新连接中"横条提示？');
    expect(hasReconnectBanner).toBe(false);
    await agent.aiAssert('房间页仍正常显示，麦位区域可见');

  } finally {
    await agent.destroy().catch(() => {});
  }
});

// ── TC-RESILIENCE-00005：进程被杀后 JWT 持久化 ───────────────────────────────

test('TC-RESILIENCE-00005: 进程被系统杀死后冷启动 - JWT 持久化 + 直接进大厅', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) test.skip();
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';
  const phoneLocal = '500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，正在测试进程被杀后 JWT 持久化冷启动',
  });

  try {
    // Step 0: Round 5 修复（方案 D）：JWT 注入绕过 UI 登录流（agent.launch() 移除，避免 HOME 闪屏）
    await resetAndroidToMainPage(adbPrefix, ANDROID_APP_ID, phone);
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // Step 1: 强杀 App（JWT 已在 DataStore 中持久化，不调用 reset）
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`, { stdio: 'pipe' });
    await new Promise(r => setTimeout(r, 2_000));

    // Step 2: 重新启动 App → 应直接进大厅（JWT 持久化）
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('页面稳定', { timeoutMs: 15_000 });
    const goesToHall = await agent.aiBoolean('是否直接进入了大厅（有房间列表，不是登录页）？');
    expect(goesToHall).toBe(true);
    await agent.aiAssert('大厅房间列表可见，无需重新登录（JWT 已持久化）');

  } finally {
    await agent.destroy().catch(() => {});
  }
});
