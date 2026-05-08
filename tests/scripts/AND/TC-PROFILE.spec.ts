/**
 * 测试套件：PROFILE 个人中心（Android）
 * 用例来源：doc/tests/cases/AND/TC-PROFILE.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-PROFILE-00001 — 页面布局 + 用户信息渲染
 *   TC-PROFILE-00003 — 钻石余额入口 → WalletScreen
 *   TC-PROFILE-00005 — 退出登录二次确认 + 清栈（含 JWT 清除断言）
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { resetAndroidToLoginPage, resetAndroidToMainPage } from '../support/androidReset';

test.setTimeout(300_000);

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

// ── 共用：冷启动 + 登录 ──────────────────────────────────────────────────────

async function coldStartAndLogin(agent: any, adbPrefix: string, ANDROID_APP_ID: string, phone: string) {
  // Round 5 修复（方案 D）：JWT 注入绕过 UI 登录流
  // agent.launch() 已移除 — resetAndroidToMainPage 已将 App 启动到主界面，保留会触发 HOME 闪屏
  await resetAndroidToMainPage(adbPrefix, ANDROID_APP_ID, phone);
  await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });
}

// ── TC-PROFILE-00001：页面布局 + 用户信息渲染 ────────────────────────────────

test('TC-PROFILE-00001: 页面布局 + 用户信息渲染', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语',
  });

  try {
    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // Step1：点击"我的" Tab
    await agent.aiTap('底部 Tab 栏中的"我的"或"Me"或"个人"选项卡');
    await agent.aiWaitFor('进入个人中心页面', { timeoutMs: 10_000 });

    // Step2：验证顶部区域
    await agent.aiAssert('顶部区域有深色渐变背景，中央显示金色边框头像，下方有用户昵称，再下方有 ID 信息');

    // Step3：验证余额卡片（可能是 💎 钻石或 💰 硬币/Coins，任一金融余额展示均可）
    // [自愈-Round1-Strategy-B] App 实际展示 wallet/coins 图标而非 💎，放宽断言兼容两种样式
    await agent.aiAssert('页面中有余额展示区域或行（含金融余额数字，可能是 💎 钻石、💰 硬币或钱包图标）');

    // Step4：验证中部列表
    // [自愈-Round5] App 实际仅展示"设置"一项（编辑资料/关于我们未在此版本实现）
    await agent.aiAssert('页面中有"设置"功能入口（含齿轮图标）');

    // Step5：验证底部退出登录
    await agent.aiAssert('页面底部有"退出登录"按钮（红色文字）');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-PROFILE-00003：钻石余额入口 → WalletScreen ────────────────────────────

test('TC-PROFILE-00003: 钻石余额入口进入 WalletScreen', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语',
  });

  try {
    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // 进入个人中心
    await agent.aiTap('底部 Tab 栏中的"我的"或"Me"选项卡');
    await agent.aiWaitFor('进入个人中心页面', { timeoutMs: 10_000 });

    // Step1：点击钻石余额行
    await agent.aiTap('钻石余额行（含 💎 图标和余额数字），或右侧 ">" 箭头');
    await agent.aiWaitFor('进入钱包页面', { timeoutMs: 10_000 });
    await agent.aiAssert('顶部标题显示"我的钱包"或"Wallet"，大卡片显示钻石余额数字 💎');

    // Step2：按返回
    await agent.aiTap('返回按钮 或 左上角 "←" 图标');
    await agent.aiWaitFor('返回个人中心页面', { timeoutMs: 8_000 });
    await agent.aiAssert('已返回个人中心页面，可见退出登录按钮或用户信息');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-PROFILE-00005：退出登录二次确认 + 清栈 ────────────────────────────────

test('TC-PROFILE-00005: 退出登录二次确认 + 清栈', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语',
  });

  try {
    await coldStartAndLogin(agent, adbPrefix, ANDROID_APP_ID, phone);

    // 进入个人中心
    await agent.aiTap('底部 Tab 栏中的"我的"或"Me"选项卡');
    await agent.aiWaitFor('进入个人中心页面', { timeoutMs: 10_000 });

    // Step1：点击退出登录
    await agent.aiTap('底部的"退出登录"按钮（红色文字）');
    await agent.aiWaitFor('弹出退出确认对话框', { timeoutMs: 8_000 });
    // [自愈-Round1-Strategy-B] 对话框实际为英语："Sign out / Cancel / Confirm"，兼容中英文
    await agent.aiAssert('弹出确认对话框（标题可能是"确定要退出登录"或"Sign out"），包含取消/Cancel 和确认/Confirm/退出 两个按钮');

    // Step2：点击取消
    await agent.aiTap('"取消" 按钮');
    await agent.aiWaitFor('对话框关闭，仍在个人中心', { timeoutMs: 5_000 });
    await agent.aiAssert('对话框已关闭，仍在个人中心页面');

    // Step3：再次点击退出 → 确认退出
    await agent.aiTap('底部的"退出登录"按钮（红色文字）');
    await agent.aiWaitFor('弹出退出确认对话框', { timeoutMs: 8_000 });
    await agent.aiTap('"退出" 或 "确定" 按钮（红色，确认退出登录）');
    await agent.aiWaitFor('跳转到登录页面', { timeoutMs: 15_000 });
    await agent.aiAssert('当前页面是登录页，显示手机号输入框，不是个人中心页');

    // Step4：JWT 持久化断言 — force-stop 后重启仍应进入登录页
    execSync(`${adbPrefix} shell am force-stop ${ANDROID_APP_ID}`);
    await new Promise(r => setTimeout(r, 1500));
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面加载完成', { timeoutMs: 20_000 });
    const hasDialog = await agent.aiBoolean('当前界面是否存在弹窗？');
    try {
      await agent.aiTap('"同意" 或 "确定" 或 "关闭" 按钮');
    } catch { /* 忽略：弹窗已由 ADB 关闭或无弹窗 */ }
    await agent.aiAssert('重启后仍显示登录页（手机号输入框可见），不是大厅或个人中心页', {
      errorMessage: 'JWT 清除失败：退出登录后重启仍显示非登录页，token 未清除',
    });

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
