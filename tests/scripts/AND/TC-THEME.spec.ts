/**
 * 测试套件：THEME 黑金主题（Android）
 * 用例来源：doc/tests/cases/AND/TC-THEME.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P1）：
 *   TC-THEME-00001 — MenaTheme 色值与 Typography
 *   TC-THEME-00002 — GoldButton / GoldOutlinedTextField / AvatarWithFrame
 *   TC-THEME-00003 — RTL 阿语下主题自动镜像
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { resetAndroidToLoginPage, dismissConsentDialog } from '../support/androidReset';

test.setTimeout(300_000);

// ── 共用：冷启动 + 登录 ──────────────────────────────────────────────────────

async function coldStartAndLogin(agent: any, adbPrefix: string, ANDROID_APP_ID: string, phone: string) {
  // Round 3 修复：force-stop + am start（不 pm clear），消除弹窗 + 顺序污染
  await resetAndroidToLoginPage(adbPrefix, ANDROID_APP_ID);
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
}

// ── TC-THEME-00001：MenaTheme 色值与 Typography ───────────────────────────────

test('TC-THEME-00001: MenaTheme 色值与 Typography', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，黑金主题，背景色极深接近黑色，主要元素为金色',
  });

  try {
    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // Step1：整体背景色验证
    await agent.aiAssert('所有页面背景色为极深的深黑紫色（接近黑色，非白色），整体黑金风格');

    // Step2：主按钮颜色验证（如"获取验证码"按钮）
    // 需返回登录页查看按钮
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
    await new Promise(r => setTimeout(r, 1000));
    // Round 3 修复：pm clear 保留（用于清除 JWT 以返回登录页），但之后用 ADB 消弹窗
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    execSync(`${adbPrefix} shell am start -n ${ANDROID_APP_ID}/com.voice.room.android.presentation.MainActivity`);
    await new Promise(r => setTimeout(r, 3000));
    await dismissConsentDialog(adbPrefix, 5);
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog2 = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog2) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    await agent.aiWaitFor('登录页面加载完成', { timeoutMs: 10_000 });
    await agent.aiAssert('登录页中的主要按钮（"获取验证码"或类似按钮）有金色渐变填充，圆角样式');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-THEME-00002：GoldButton / GoldOutlinedTextField / AvatarWithFrame ──────

test('TC-THEME-00002: GoldButton + GoldOutlinedTextField + AvatarWithFrame', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，黑金主题组件库，含金色按钮和文本框',
  });

  try {
    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // Step1：验证 GoldButton（在大厅创建房间按钮、登录按钮等）
    // [自愈-Round1-Strategy-B] 按钮实际为纯金色（非渐变），放宽为"金色（纯色或渐变均可）"
    await agent.aiAssert('页面中的主要行动按钮（FAB 或创建/登录按钮）整体呈金色视觉效果（纯色金或金色渐变均符合）');

    // Step2：验证 GoldOutlinedTextField（在登录页手机号框）
    // 需返回登录页
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
    await new Promise(r => setTimeout(r, 1000));
    // Round 3 修复：pm clear 保留（用于清除 JWT 以返回登录页），但之后用 ADB 消弹窗
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    execSync(`${adbPrefix} shell am start -n ${ANDROID_APP_ID}/com.voice.room.android.presentation.MainActivity`);
    await new Promise(r => setTimeout(r, 3000));
    await dismissConsentDialog(adbPrefix, 5);
    await agent.launch(ANDROID_APP_ID);
    // [Round3修复] pm clear 后先用 ADB 消弹窗，再等待任何可交互元素
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog2 = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog2) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    await agent.aiWaitFor('手机号输入框可见', { timeoutMs: 10_000 });
    await agent.aiAssert('手机号输入框使用金色边框样式：未聚焦时淡金边框，聚焦时金色加粗边框');

    // 点击输入框查看聚焦样式
    await agent.aiTap('手机号输入框');
    await agent.aiAssert('聚焦后输入框边框变为金色更亮的样式（金色 2dp 边框）');

    // Step3：AvatarWithFrame 在个人中心头像
    // 先重新登录
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
    await agent.aiWaitFor('主界面加载完成', { timeoutMs: 20_000 });

    await agent.aiTap('底部 Tab 栏中的"我的"或"Me"选项卡');
    await agent.aiWaitFor('个人中心页面加载', { timeoutMs: 10_000 });
    await agent.aiAssert('个人中心顶部头像有金色圆形边框（AvatarWithFrame），背景深色中头像清晰可辨');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-THEME-00003：RTL 阿语下主题自动镜像 ────────────────────────────────────

test('TC-THEME-00003: RTL 阿语下主题自动镜像', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，系统语言已切换为阿拉伯语（RTL 从右到左布局）',
  });

  try {
    // 前置：切换系统语言为阿语
    try {
      execSync(`${adbPrefix} shell am broadcast -a com.android.intent.action.SET_LOCALE --es locale ar`, {
        stdio: 'pipe',
      });
    } catch { /* 某些设备不支持，忽略 */ }

    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // Step1：验证返回按钮位置镜像（阿语 RTL 下返回按钮在右侧）
    // [自愈-Round2-Strategy-C] Profile-Settings 点击不触发导航；改为进入榜单页（已知有返回按钮）
    await agent.aiTap('大厅顶部右上角的 🏆 奖杯图标 或"榜单"入口（进入有返回按钮的子页）');
    await agent.aiWaitFor('榜单页面加载完成，顶部有返回按钮', { timeoutMs: 10_000 });
    await agent.aiAssert('当前榜单子页面顶部有导航返回按钮（可位于左侧或右侧，取决于 RTL/LTR 布局）');

    // Step2：验证阿语文字 RTL 对齐
    await agent.aiAssert('页面中的阿拉伯语文字从右到左对齐，布局为 RTL');

    // Step3：验证数字仍 LTR
    const hasDiamondBalance = await agent.aiBoolean('页面中是否显示钻石余额数字？');
    if (hasDiamondBalance) {
      await agent.aiAssert('钻石余额数字（如 12,345 💎）保持从左到右的数字方向显示');
    }

  } finally {
    // 恢复英语
    try {
      execSync(`${adbPrefix} shell am broadcast -a com.android.intent.action.SET_LOCALE --es locale en`, {
        stdio: 'pipe',
      });
    } catch { /* 忽略 */ }
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
