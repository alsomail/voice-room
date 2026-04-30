/**
 * 测试套件：ROOM 房间大厅（Android）
 * 用例来源：doc/tests/cases/AND/TC-ROOM.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-ROOM-00001 — 大厅网格渲染 + 分页下拉
 *   TC-ROOM-00003 — 创建房间 E2E 成功（含 DB 副作用断言）
 *   TC-ROOM-00005 — 房间卡片点击进入 RoomScreen
 */
import { test, expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';

test.setTimeout(300_000);

// ── 工具函数 ─────────────────────────────────────────────────────────────────

const psql = (databaseUrl: string, sql: string): string =>
  execSync(`psql "${databaseUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe'],
  }).trim();

// ── TC-ROOM-00001：大厅网格渲染 + 分页下拉 ───────────────────────────────────

test('TC-ROOM-00001: 大厅网格渲染 + 分页下拉', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置 — 请在 tests/scripts/env/.env.local 中设置 ANDROID_APP_ID');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语',
  });

  try {
    // Step0：冷启动 + 弹窗处理
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }

    // 需要先登录才能进大厅 — 使用 seed 用户
    const phone = '+966500000900';
    const phoneLocal = '500000900';
    // 预置验证码
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiWaitFor('手机号输入框可见，登录页面已加载完成', { timeoutMs: 10_000 });
    await agent.aiInput(phoneLocal, '手机号输入框');
    await agent.aiTap('"获取验证码" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });
    // 覆写已知验证码
    try {
      redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);
    } catch (e) {
      if (!(e instanceof RedisCliUnavailableError)) throw e;
    }
    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，底部 Tab 栏可见', { timeoutMs: 20_000 });

    // Step1：进入大厅，验证标题和 FAB
    await agent.aiAssert('顶部显示"语聊房"或 VoiceRoom 标题，右下角或底部附近有金色 "+" 圆形按钮');

    // Step2：验证房间网格
    await agent.aiAssert('页面中显示房间列表或房间卡片网格，每张卡片含封面图、标题、在线人数等信息');

    // Step3：下拉刷新
    await agent.aiTap('顶部下拉刷新区域');
    await agent.aiWaitFor('页面完成刷新', { timeoutMs: 15_000 });
    await agent.aiAssert('房间列表已刷新，显示正常');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:+966500000900`, `sms:cooldown:+966500000900`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-ROOM-00003：创建房间 E2E 成功 ─────────────────────────────────────────

test('TC-ROOM-00003: 创建房间 Bottom Sheet E2E 成功', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

  const roomTitle = `e2e_room_${Date.now()}`;
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语',
  });

  try {
    // 前置清理：删除同名房间（防测试污染）
    psql(DATABASE_URL, `DELETE FROM rooms WHERE title='${roomTitle}'`);

    // Step0：冷启动 + 跳过弹窗 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });
    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }
    // 预置验证码并登录
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

    // Step1：点击创建房间 FAB
    await agent.aiTap('右下角金色 "+" FAB 按钮（创建房间入口）');
    await agent.aiWaitFor('创建房间 Bottom Sheet 已弹出', { timeoutMs: 10_000 });
    await agent.aiAssert('Bottom Sheet 标题为"创建房间"，包含房间标题输入框');

    // Step2：输入房间标题
    await agent.aiInput(roomTitle, '房间标题输入框');
    await agent.aiAssert('"创建"按钮已变为金色可点击状态');

    // Step3：点击创建，等待跳转
    await agent.aiTap('"创建" 按钮');
    await agent.aiWaitFor('已进入 RoomScreen，页面显示房间内视图', { timeoutMs: 20_000 });
    await agent.aiAssert(`顶部显示房间标题 "${roomTitle}" 或已进入房间内部视图`);

    // ── DB 副作用断言（铁律 6）───────────────────────────────────────────────
    const count = psql(DATABASE_URL, `SELECT COUNT(*) FROM rooms WHERE title='${roomTitle}'`);
    expect(count).toBe('1');

    const status = psql(DATABASE_URL, `SELECT status FROM rooms WHERE title='${roomTitle}'`);
    expect(status).toBe('active');

  } finally {
    // 清理
    try {
      psql(DATABASE_URL, `DELETE FROM rooms WHERE title='${roomTitle}'`);
    } catch { /* 忽略 */ }
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});

// ── TC-ROOM-00005：房间卡片点击进入 RoomScreen ───────────────────────────────

test('TC-ROOM-00005: 房间卡片点击进入 RoomScreen', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为阿拉伯语或英语',
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

    // Step1：确认大厅有房间卡片
    await agent.aiAssert('页面中可见至少一张房间卡片，含封面图和房间标题');

    // Step2：点击第一张房间卡片
    await agent.aiTap('第一张房间卡片（含封面图和标题）');

    // Step3：等待进入 RoomScreen
    await agent.aiWaitFor('已进入房间内页面，显示麦位区域或房间标题', { timeoutMs: 15_000 });
    await agent.aiAssert('房间页面已加载，顶部显示房间标题，可见麦位区或主播区');

    // Step4：返回大厅
    await agent.aiTap('返回按钮 或 左上角 "←" 图标');
    await agent.aiWaitFor('返回大厅，房间卡片列表可见', { timeoutMs: 10_000 });
    await agent.aiAssert('已返回大厅，房间列表可见');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
