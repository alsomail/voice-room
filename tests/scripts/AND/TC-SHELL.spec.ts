/**
 * 测试套件：SHELL 启动页 / 主 Tab 骨架（Android）
 * 用例来源：doc/tests/cases/AND/TC-SHELL.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-SHELL-00001 — SplashScreen Logo 动画 + 跳转分流
 *   TC-SHELL-00002 — MainScreen 底部 3 Tab + 状态保留
 *   TC-SHELL-00005 — RoomScreen 黑金升级 + 主副麦 + 弹幕 + 底栏
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

// ── TC-SHELL-00001：SplashScreen Logo 动画 + 跳转分流 ────────────────────────

test('TC-SHELL-00001: SplashScreen Logo 动画 + 跳转分流', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，有 SplashScreen 和登录页',
  });

  try {
    // Step1：无 JWT 冷启动 → 应跳到 LoginScreen
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    // 恢复 App 语言为中文（Android 13+ app-specific locale）
    try {
      execSync(`${adbPrefix} shell cmd locale set-app-locales ${ANDROID_APP_ID} --locales zh-CN`, { stdio: 'pipe' });
    } catch { /* 旧版 Android 不支持，忽略 */ }
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框（弹窗或登录页均可）', { timeoutMs: 20_000 });
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    await agent.aiWaitFor('手机号输入框可见，登录页面已加载完成', { timeoutMs: 10_000 });
    await agent.aiAssert('当前显示登录页面，有手机号输入框（Splash 动画完成后跳转）');

    // Step2：有效 JWT 冷启动 → 应跳到 MainScreen
    // 先登录获取 JWT
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('500000900', '手机号输入框');
    await agent.aiTap('"获取验证码"/"Get Code"/"احصل على الرمز" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });

    // force-stop 后重启（保留 JWT）
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
    await new Promise(r => setTimeout(r, 1500));
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面加载完成', { timeoutMs: 20_000 });
    const hasDialog = await agent.aiBoolean('当前界面是否存在弹窗？');
    if (hasDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "关闭" 按钮');
    }
    await agent.aiAssert('有效 JWT 重启后直接进入主界面（大厅列表可见），不是登录页', {
      errorMessage: 'JWT 持久化失败：force-stop 后重启仍显示登录页',
    });

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-SHELL-00002：MainScreen 底部 3 Tab + 状态保留 ──────────────────────────

test('TC-SHELL-00002: MainScreen 底部 3 Tab + 状态保留', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，主界面有底部 3 Tab：房间/消息/我的',
  });

  try {
    // 冷启动 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    // 恢复 App 语言为中文（Android 13+ app-specific locale）
    try {
      execSync(`${adbPrefix} shell cmd locale set-app-locales ${ANDROID_APP_ID} --locales zh-CN`, { stdio: 'pipe' });
    } catch { /* 旧版 Android 不支持，忽略 */ }
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
    await agent.aiTap('"获取验证码"/"Get Code"/"احصل على الرمز" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });

    // Step1：验证 3 个 Tab
    await agent.aiAssert('底部有 3 个 Tab：房间（🏠 或"房间"）、消息（💬 或"消息"）、我的（👤 或"我的"），当前"房间" Tab 为金色选中状态');

    // Step2：切换到"消息" Tab
    await agent.aiTap('底部"消息" Tab');
    await agent.aiWaitFor('消息页面加载', { timeoutMs: 8_000 });
    await agent.aiAssert('消息页显示占位文案（如"消息功能即将上线"）或消息列表');

    // Step3：切换到"我的" Tab
    await agent.aiTap('底部"我的" Tab');
    await agent.aiWaitFor('个人中心页面加载', { timeoutMs: 8_000 });
    await agent.aiAssert('个人中心页面已加载，显示用户头像和昵称');

    // Step4：切回"房间" Tab
    await agent.aiTap('底部"房间" Tab');
    await agent.aiWaitFor('房间大厅页面已显示', { timeoutMs: 8_000 });
    await agent.aiAssert('房间大厅页面可见，房间列表已显示');

    // Step5：未选中 Tab 颜色验证
    await agent.aiAssert('当前"房间" Tab 图标或文字为金色（选中），其余 Tab 为灰色（未选中）');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-SHELL-00005：RoomScreen 黑金升级 + 主副麦 + 弹幕 + 底栏 ─────────────

test('TC-SHELL-00005: RoomScreen 黑金升级 + 主副麦 + 弹幕 + 底栏', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面黑金主题，房间内有主播麦和副麦位网格',
  });

  try {
    // 冷启动 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    // 恢复 App 语言为中文（Android 13+ app-specific locale）
    try {
      execSync(`${adbPrefix} shell cmd locale set-app-locales ${ANDROID_APP_ID} --locales zh-CN`, { stdio: 'pipe' });
    } catch { /* 旧版 Android 不支持，忽略 */ }
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
    await agent.aiTap('"获取验证码"/"Get Code"/"احصل على الرمز" 按钮');
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

    // Step1：验证顶部主播麦区
    await agent.aiAssert('顶部区域有主播麦位，含金色光晕头像和房间标题');

    // Step2：验证副麦网格
    await agent.aiAssert('中部有麦位网格，含多个副麦位（空位显示 "+" 或已占位显示头像）');

    // Step3：验证底部操作栏
    await agent.aiAssert('底部操作栏从左至右包含：麦克风（🎤）、礼物（🎁）、关注（❤️）、离开（🚪）等功能按钮');

    // Step4：测试离开房间
    await agent.aiTap('底部操作栏的"离开"按钮（🚪 图标）');
    await agent.aiWaitFor('弹出离开确认对话框', { timeoutMs: 8_000 });
    await agent.aiAssert('弹出"确定离开房间？"对话框，有取消和确认两个按钮');

    // 点击取消（不实际离开）
    await agent.aiTap('"取消" 按钮');
    await agent.aiWaitFor('对话框关闭，仍在房间内', { timeoutMs: 5_000 });
    await agent.aiAssert('仍在房间内，麦位布局和底部操作栏可见');

    // 点击离开确认
    await agent.aiTap('底部操作栏的"离开"按钮（🚪 图标）');
    await agent.aiWaitFor('弹出离开确认对话框', { timeoutMs: 8_000 });
    await agent.aiTap('"确定" 或 "离开" 按钮（确认离开）');
    await agent.aiWaitFor('已返回大厅，房间列表可见', { timeoutMs: 10_000 });
    await agent.aiAssert('已返回大厅页面，可见房间卡片列表');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
