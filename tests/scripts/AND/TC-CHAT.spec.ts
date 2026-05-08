/**
 * 测试套件：CHAT 聊天 & WS（Android）
 * 用例来源：doc/tests/cases/AND/TC-CHAT.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-CHAT-00002 — 公屏发送 + 接收 + 自动滚动
 */
import { test, expect } from '../support/fixtures';
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

// ── TC-CHAT-00002：公屏发送 + 接收 + 自动滚动 ────────────────────────────────

test('TC-CHAT-00002: 公屏发送 + 接收 + 自动滚动', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置 — 请在 tests/scripts/env/.env.local 中设置 ANDROID_APP_ID');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

  const phone = '+966500000900';
  const phoneLocal = '500000900';
  const testMsg = `hello_e2e_${Date.now()}`;
  const ROOM_ID = e2eEnv.roomId as string | undefined;

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语，房间内有公屏聊天区域',
  });

  try {
    // 前置：标准化重置（force-stop + am start，不 pm clear 避免弹窗）+ 登录
    await resetAndroidToLoginPage(adbPrefix, ANDROID_APP_ID, 5, true);
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
    await agent.aiInput(phoneLocal, '手机号输入框');
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

    // Step1：进入房间
    await agent.aiTap('第一张房间卡片或已知 E2E 测试房间卡片');
    await agent.aiWaitFor('已进入房间，可见公屏聊天区域或麦位布局', { timeoutMs: 15_000 });

    // Step2：点击底部聊天输入框
    await agent.aiTap('底部聊天输入框或消息输入区域');
    await agent.aiWaitFor('键盘弹起，输入框聚焦', { timeoutMs: 8_000 });

    // Step3：输入消息
    await agent.aiInput(testMsg, '聊天输入框');

    // Step4：发送
    await agent.aiTap('"发送" 按钮或发送图标');

    // Self-Healing Round6：等待3秒让WS广播到达并渲染；再用更宽泛的描述等待聊天气泡
    await new Promise(r => setTimeout(r, 3000));

    // KNOWN: BUG-CHAT-WS-001 - WS 广播未完全实现，聊天消息可能不显示在公屏
    // Step5：断言消息出现在公屏（宽泛检测：任意文字消息气泡存在即可）
    await agent.aiWaitFor(`聊天区域出现刚发送的消息气泡（包含任何文字内容）`, { timeoutMs: 15_000 });
    await agent.aiAssert(`公屏列表底部可见刚发送的消息，内容包含 "${testMsg.slice(0, 20)}"`);

    // Step6：长按消息验证菜单（T-30053 self-healing: aiTap→aiLongPress，T-30053 已实现 combinedClickable onLongClick）
    await agent.aiLongPress(`包含 "${testMsg.slice(0, 20)}" 的聊天气泡`);
    await agent.aiWaitFor('弹出操作菜单', { timeoutMs: 8_000 });
    await agent.aiAssert('操作菜单中包含"复制"选项');

    // 关闭菜单
    const hasMenu = await agent.aiBoolean('当前是否有弹出菜单或对话框？');
    if (hasMenu) {
      await agent.aiTap('"关闭" 或点击空白区域关闭菜单');
    }

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
