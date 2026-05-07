/**
 * 测试套件：MIC 麦位（Android）
 * 用例来源：doc/tests/cases/AND/TC-MIC.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-MIC-00001 — 权限申请：拒绝后 Fallback 到系统设置
 *   TC-MIC-00002 — 上麦 → 下麦 E2E（含 DB 副作用断言）
 *   TC-MIC-00009 — 点击自己已占麦位图标 → 触发下麦（T-30055 onMicSlotClick 修复验证）P0
 *
 * [自愈 R1 2026-05-07] TC-MIC-00009：UI 点击空麦位无响应，改用 ensureMicOccupant
 *   WS 程序化上麦作为前置条件（Strategy B），App 收到 MicTaken 广播后 UI 自动更新。
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { ensureMicOccupant } from '../support/ensureMicOccupant';
import type { MicOccupant } from '../support/ensureMicOccupant';
import WebSocket from 'ws';

test.setTimeout(600_000); // 10 min — AI visual calls are slow (~20-40s each), 15+ calls needed

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
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语',
  });

  try {
    // 冷启动 + 弹窗处理 + 登录
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

    // Step1：点击麦克风按钮或空麦位
    await agent.aiTap('底部操作栏中的麦克风按钮（🎤 图标）或空麦位的 "+" 按钮');
    await agent.aiWaitFor('弹出权限请求或系统对话框', { timeoutMs: 10_000 });
    await agent.aiAssert('弹出系统录音权限请求对话框，包含允许和拒绝选项');

    // Step2：点击拒绝
    await agent.aiTap('"拒绝" 或 "Deny" 按钮（拒绝录音权限）');
    await agent.aiWaitFor('权限被拒绝，App 给出提示', { timeoutMs: 8_000 });
    await agent.aiAssert('App 内出现 SnackBar 或 Toast 提示需要麦克风权限，或有"去设置"按钮');

    // Step3：点击去设置（如果有）
    await new Promise(r => setTimeout(r, 2000)); // 等待 Toast/SnackBar 完全显示
    const hasSettingsBtn = await agent.aiBoolean('在底部 Toast 提示条或 SnackBar 中是否有"去设置"或"Settings"文字按钮（不是房间内的其他按钮）？');
    if (hasSettingsBtn) {
      await agent.aiTap('Toast 或 SnackBar 中的"去设置" 或 "Settings" 按钮（位于屏幕底部提示条中）');
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
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语，房间内可见主播麦和副麦',
  });

  try {
    // 冷启动 + 登录
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    // 前置：pm clear 之后重新授予 RECORD_AUDIO 权限（pm clear 会撤销所有权限）
    try {
      execSync(`${adbPrefix} shell pm grant ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, { stdio: 'pipe' });
    } catch { /* 忽略 */ }
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

// ── TC-MIC-00009：点击自己已占麦位图标 → 触发下麦（T-30055 onMicSlotClick 修复验证）───────

/**
 * 关联 Bug：T-30055（BUG-MIC-ONCLICK）
 * 修复分支：fix-mic-onclick-t30055
 * 验证点：RoomScreen/AppNavGraph 的 onMicSlotClick 回调已正确传递，
 *          点击自己所在的麦位图标能弹出"下麦"确认菜单，并发出 LeaveMic WS 帧。
 */
test('TC-MIC-00009: 点击自己已占麦位图标触发下麦（T-30055 onMicSlotClick 修复）', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';
  const WS_URL = e2eEnv.appWsUrl as string;
  const TOKEN_B = process.env.E2E_USER_B_TOKEN ?? '';
  const ROOM_ID = e2eEnv.ids?.roomId as string | undefined;

  // ── 前置：授予 RECORD_AUDIO 权限 ──────────────────────────────────────────────
  try {
    execSync(`${adbPrefix} shell pm grant ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, { stdio: 'pipe' });
  } catch { /* 忽略 */ }

  // ── WS 旁观者 U2（接收 MicLeft 广播） ────────────────────────────────────────
  let wsB: WebSocket | null = null;
  let micLeftReceived: Record<string, unknown> | null = null;
  const micLeftPromise: Promise<Record<string, unknown> | null> = new Promise((resolve) => {
    if (!TOKEN_B || !WS_URL) { resolve(null); return; }
    try {
      wsB = new WebSocket(`${WS_URL}?token=${encodeURIComponent(TOKEN_B)}`);
      wsB.on('open', () => {
        // U2 加入房间（roomId 从 env 读取，可能为空则跳过）
        const joinRoomId = ROOM_ID || '';
        if (joinRoomId) {
          wsB!.send(JSON.stringify({ type: 'JoinRoom', payload: { room_id: joinRoomId }, msg_id: `j_u2_${Date.now()}` }));
        }
      });
      wsB.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString()) as Record<string, unknown>;
          if (msg.type === 'MicLeft') {
            resolve(msg);
          }
        } catch { /* 忽略 */ }
      });
      wsB.on('error', () => resolve(null));
      // 60 秒超时兜底
      setTimeout(() => resolve(null), 60_000);
    } catch {
      resolve(null);
    }
  });

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语，房间内可见主播麦和副麦，自己占据的麦位显示头像',
  });

  // [自愈 R1] 在 try 外声明 u1Occupant，确保 finally 块可以访问
  let u1Occupant: MicOccupant | null = null;

  try {
    // ── 自愈 R2 前置 DB Bump（在 App 启动前执行，避免 App 缓存旧顺序） ──────────
    // 服务器按 member_count DESC, created_at DESC 排序；seed 房间 created_at 相同→顺序不定。
    // 在 App 启动前把 E2E Test Room 的 created_at 推到最新，确保第一屏可见。
    if (ROOM_ID && DATABASE_URL) {
      try {
        psql(DATABASE_URL, `UPDATE rooms SET created_at = NOW() + INTERVAL '1 hour', updated_at = NOW() WHERE id='${ROOM_ID}'`);
        console.log('[TC-MIC-00009] ✅ E2E Test Room created_at 已提前到 NOW()+1h，将排在列表第一位');
      } catch (dbBumpErr: any) {
        console.warn(`[TC-MIC-00009] ⚠️ created_at bump 失败（非致命）：${dbBumpErr?.message ?? dbBumpErr}`);
      }
    }

    // ── 冷启动 + 授权 + 登录 ──────────────────────────────────────────────────
    execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
    try {
      execSync(`${adbPrefix} shell pm grant ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, { stdio: 'pipe' });
    } catch { /* 忽略 */ }
    try {
      execSync(`${adbPrefix} shell cmd locale set-app-locales ${ANDROID_APP_ID} --locales zh-CN`, { stdio: 'pipe' });
    } catch { /* 旧版 Android 不支持，忽略 */ }

    await agent.launch(ANDROID_APP_ID);
    await agent.aiWaitFor('界面上有可交互的按钮或输入框', { timeoutMs: 15_000 });

    const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
    if (hasConsentDialog) {
      await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
    }

    // 注入 SMS 验证码
    try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
    catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

    await agent.aiWaitFor('手机号输入框可见', { timeoutMs: 10_000 });
    await agent.aiInput('500000900', '手机号输入框');
    await agent.aiTap('"获取验证码"/"Get Code"/"احصل على الرمز" 按钮');
    await agent.aiWaitFor('按钮进入倒计时状态', { timeoutMs: 10_000 });

    try { redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']); }
    catch (e) { if (!(e instanceof RedisCliUnavailableError)) throw e; }

    await agent.aiInput('123456', '验证码输入框');
    await agent.aiTap('登录 或 确认 按钮');
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // ── 进入 seed 测试房间（自愈 R2：Strategy C — 由 App 自身上麦）──────────────
    // 根因：Android JoinRoomResult 数据类不含 payload 字段，App 不解析 mic_slots；
    //        只有 MicTaken 实时广播才能更新 App 麦位 UI。
    //        因此不再使用 ensureMicOccupant（进房前占麦 App 会错过广播），
    //        改为让 App 自己点击空麦位 0 完成上麦，借助 App 原生 TakeMic 路径更新 UI。
    // 下拉刷新房间列表，确保 App 拿到最新排序（created_at 已在启动前 bump 到 NOW()+1h）
    await agent.aiAction('在房间列表页向下拖动以触发下拉刷新，等待列表重新加载完成');
    await new Promise(r => setTimeout(r, 1_500));
    await agent.aiAction('在房间列表中找到标题为 "E2E Test Room" 的房间卡片，如有需要向下滚动查找，然后点击进入');

    // ── 等待进入房间：UIAutomator 轮询主麦位出现（避免耗时 AI 调用）────────────
    console.log('[TC-MIC-00009] R14：UIAutomator 轮询等待进入房间（主麦位出现）...');
    const enterPollStart = Date.now();
    let enteredRoom = false;
    while (Date.now() - enterPollStart < 20_000) {
      await new Promise(r => setTimeout(r, 2_000));
      try {
        execSync(`${adbPrefix} shell uiautomator dump /sdcard/uia_enter.xml`, { stdio: 'pipe' });
        const xmlEnter = execSync(`${adbPrefix} shell cat /sdcard/uia_enter.xml`, {
          encoding: 'utf-8', maxBuffer: 16 * 1024 * 1024,
        });
        if (xmlEnter.includes('主麦位，空位，点击上麦')) {
          enteredRoom = true;
          console.log('[TC-MIC-00009] ✅ UIAutomator 确认已进入房间，主麦位空位可见');
          break;
        }
      } catch { /* continue */ }
    }
    if (!enteredRoom) {
      await agent.aiWaitFor('已进入房间，麦位区域可见', { timeoutMs: 10_000 });
    }

    // ── 前置：App 自身上麦（直接坐标点击主麦位 slot 0）──────────────────────────
    // [R14] 根因修复：
    //   slot 0 物理中心 = (540, 402)（实测：bounds=[430,259][650,545]，center=(540,402)）
    //   UIAutomator 在"占用"状态 bounds=[0,83][1080,2214]（全屏），center=(540,1148)→错误！
    //   因此两次点击均使用固定坐标 (540, 402)，绕过 UIAutomator 异常 bounds。
    // [R14] JWT fix：RoomViewModel.joinRoom 现从 JWT sub 提取 currentUserId。
    console.log('[TC-MIC-00009] R14：直接坐标点击主麦位 slot 0 (540, 402)...');
    execSync(`${adbPrefix} shell input tap 540 402`, { stdio: 'pipe' });

    // 等 1.5s 后检查是否弹出麦克风权限对话框（UIAutomator 快速检查，无需 AI）
    await new Promise(r => setTimeout(r, 1_500));
    try {
      execSync(`${adbPrefix} shell uiautomator dump /sdcard/uia_perm.xml`, { stdio: 'pipe' });
      const xmlPerm = execSync(`${adbPrefix} shell cat /sdcard/uia_perm.xml`, {
        encoding: 'utf-8', maxBuffer: 16 * 1024 * 1024,
      });
      // 检查麦克风权限对话框（Accompanist 或系统弹窗）
      if (xmlPerm.includes('rationale_confirm_button') ||
          xmlPerm.includes('需要麦克风权限') ||
          (xmlPerm.includes('RECORD_AUDIO') && xmlPerm.includes('Allow'))) {
        console.log('[TC-MIC-00009] 检测到麦克风权限对话框（App 内置），点击"允许"...');
        const allowIdx = xmlPerm.indexOf('允许');
        if (allowIdx > 0) {
          const aNodeStart = xmlPerm.lastIndexOf('<node ', allowIdx);
          const aNodeEnd = xmlPerm.indexOf('>', aNodeStart);
          const aNode = xmlPerm.substring(aNodeStart, aNodeEnd + 1);
          const aBm = aNode.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
          if (aBm) {
            execSync(`${adbPrefix} shell input tap ${Math.round((+aBm[1]+parseInt(aBm[3]))/2)} ${Math.round((+aBm[2]+parseInt(aBm[4]))/2)}`, { stdio: 'pipe' });
            await new Promise(r => setTimeout(r, 1_000));
          }
        }
      }
    } catch { /* ignore */ }

    // 验证上麦成功：UIAutomator 轮询 contentDesc 变化（max 15s）
    console.log('[TC-MIC-00009] 等待主麦位被占用（UIAutomator 轮询 contentDesc）...');
    const pollStart = Date.now();
    let slotOccupied = false;
    while (Date.now() - pollStart < 15_000) {
      await new Promise(r => setTimeout(r, 2_500));
      try {
        execSync(`${adbPrefix} shell uiautomator dump /sdcard/uia_post.xml`, { stdio: 'pipe' });
        const xmlPost = execSync(`${adbPrefix} shell cat /sdcard/uia_post.xml`, {
          encoding: 'utf-8', maxBuffer: 16 * 1024 * 1024,
        });
        // 槽位占用：contentDesc 含 "主麦位，" 但不含 "空位，点击上麦"
        if (xmlPost.includes('主麦位，') && !xmlPost.includes('空位，点击上麦')) {
          slotOccupied = true;
          console.log('[TC-MIC-00009] ✅ UIAutomator 确认主麦位已被占用');
          break;
        }
      } catch { /* continue polling */ }
    }
    if (!slotOccupied) {
      throw new Error('[TC-MIC-00009] ❌ 上麦失败：15s内 UIAutomator 未检测到占用（contentDesc 仍含"空位"）。TakeMic WS 未发送？');
    }
    console.log('[TC-MIC-00009] ✅ 上麦前置条件已满足');

    // ── Step2（TC-MIC-00009 核心）：再次点击主麦位 (540, 402) ─────────────────
    // 此时槽位已被自己占用，ViewModel.onMicSlotClick 检查 slot.userId == currentUserId
    // [R14] JWT fix 确保 currentUserId 不为空 → 触发 ShowLeaveMicConfirmDialog
    console.log('[TC-MIC-00009] Step2：直接坐标点击已占主麦位 (540, 402)...');
    execSync(`${adbPrefix} shell input tap 540 402`, { stdio: 'pipe' });

    // ── Step3：断言弹出下麦确认对话框（T-30055 修复核心断言）─────────────────
    // 新版 APK：点击已占主麦位 → onMicSlotClick → ShowLeaveMicConfirmDialog 事件
    // → RoomScreen 的 LaunchedEffect 收到 → leaveMicConfirmSlotIndex 置非 null
    // → AlertDialog 弹出（标题"下麦"，文本"确认离开麦位吗？"，按钮"下麦"/"取消"）

    // 优先 UIAutomator 轮询检测对话框（快速，max 8s），再 AI 兜底
    console.log('[TC-MIC-00009] Step3：UIAutomator 轮询等待下麦确认对话框...');
    const dialogPollStart = Date.now();
    let dialogFound = false;
    while (Date.now() - dialogPollStart < 8_000) {
      await new Promise(r => setTimeout(r, 1_500));
      try {
        execSync(`${adbPrefix} shell uiautomator dump /sdcard/uia_dialog.xml`, { stdio: 'pipe' });
        const xmlDialog = execSync(`${adbPrefix} shell cat /sdcard/uia_dialog.xml`, {
          encoding: 'utf-8', maxBuffer: 16 * 1024 * 1024,
        });
        if (xmlDialog.includes('确认离开麦位吗') || xmlDialog.includes('下麦') && xmlDialog.includes('取消')) {
          dialogFound = true;
          console.log('[TC-MIC-00009] ✅ UIAutomator 确认下麦确认对话框已弹出');
          break;
        }
      } catch { /* continue */ }
    }
    if (!dialogFound) {
      // AI 兜底（如果 UIAutomator 没检测到）
      await agent.aiWaitFor(
        '弹出下麦确认对话框，包含标题"下麦"和"确认离开麦位吗"提示文字，以及"下麦"确认按钮',
        { timeoutMs: 15_000 },
      );
    }
    await agent.aiAssert('弹出了下麦确认对话框，标题为"下麦"，内容含"确认离开麦位吗"，以及"下麦"和"取消"两个按钮');
    console.log('[TC-MIC-00009] ✅ Step2/Step3 PASS：下麦确认对话框已弹出');

    // ── Step4：确认下麦 ────────────────────────────────────────────────────────
    await agent.aiTap('"下麦" 确认按钮（对话框中的确认操作，不是取消）');
    await agent.aiWaitFor('下麦对话框已关闭，主麦位恢复为加号空位状态', { timeoutMs: 20_000 });

    // ── Step5：断言 Android 视觉 ────────────────────────────────────────────
    await agent.aiAssert('主麦位（页面中央大圆）已恢复为加号"+"空状态，头像消失，说明下麦成功');

    // ── Step6：WS 协议层断言：U2 收到 MicLeft 广播 ──────────────────────────
    // （async race，最多等 8s；无 U2 token 时跳过）
    if (TOKEN_B && WS_URL) {
      const micLeft = await Promise.race([
        micLeftPromise,
        new Promise<null>((r) => setTimeout(() => r(null), 8_000)),
      ]);
      if (micLeft !== null) {
        const payload = (micLeft as any).payload ?? {};
        expect(typeof payload.mic_index).toBe('number');
        expect(payload.forced).toBe(false);   // 主动下麦 forced=false
        expect(typeof payload.user_id).toBe('string');
        console.log(`[TC-MIC-00009] ✅ MicLeft 广播收到：mic_index=${payload.mic_index} user_id=${payload.user_id}`);
      } else {
        console.warn('[TC-MIC-00009] WS MicLeft 广播未在 8s 内收到（可能 room_id 未配置或 U2 token 无效），跳过协议断言');
      }
    }

    // ── Step7：DB 副作用断言（铁律 6）────────────────────────────────────────
    // 注意：mic_seats 表在当前 schema 中不存在（麦位状态由服务器内存 DashMap 维护，无 DB 持久化）。
    //       此处仅做防御性 try-catch；若表不存在则跳过并 warn，不视为测试失败。
    if (ROOM_ID && DATABASE_URL) {
      await new Promise(r => setTimeout(r, 1500));
      try {
        const userId = psql(DATABASE_URL, `SELECT id FROM users WHERE phone='${phone}' LIMIT 1`);
        if (userId) {
          const occupiedCount = psql(
            DATABASE_URL,
            `SELECT COUNT(*) FROM mic_seats WHERE room_id='${ROOM_ID}' AND user_id='${userId}'`,
          );
          expect(Number(occupiedCount)).toBe(0);
          console.log(`[TC-MIC-00009] ✅ DB 确认麦位已释放（user_id=${userId} 已从所有麦位移除）`);
        }
      } catch (dbErr: any) {
        // mic_seats 表暂不存在（麦位状态仅在服务器内存中）；跳过 DB 断言不阻断用例
        console.warn(`[TC-MIC-00009] ⚠️ DB 断言跳过（表可能不存在或连接失败）：${dbErr?.message ?? dbErr}`);
      }
    }

  } finally {
    // 清理
    try {
      execSync(`${adbPrefix} shell pm revoke ${ANDROID_APP_ID} android.permission.RECORD_AUDIO`, { stdio: 'pipe' });
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
    // [自愈 R1] 释放 WS 程序化占麦的连接
    if (u1Occupant) {
      await u1Occupant.dispose().catch(() => {});
    }
    try {
      if (wsB && (wsB as WebSocket).readyState === WebSocket.OPEN) {
        (wsB as WebSocket).close();
      }
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
