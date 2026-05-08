/**
 * androidReset.ts — 标准化 Android 测试前重置序列
 *
 * Round 5 P0 修复（方案 D）：JWT 注入登录，彻底跳过 UI 登录流
 *
 * 核心策略：
 *   - force-stop → API 登录获取 JWT → 编码为 DataStore proto → run-as 写入设备
 *   - am start → App 读取 JWT → 直接进入主界面（跳过登录页）
 *   - 解决 Round 4 根因：ADB tap 坐标因键盘状态而偏移、Midscene 误识"登录"标题为按钮
 *
 * Round 4 P0 修复（方案 C）：不清数据 → 只删 JWT token → 无弹窗直达登录页
 *
 * 核心策略：
 *   - force-stop → run-as 删除 auth.preferences_pb（只清 JWT，保留同意标记）
 *   - am start → App 无 JWT 直接显示登录页，无数据收集弹窗
 *   - detectScreenState 检测当前页面，若仍在主界面则二次删 token 重启
 *   - 整体不依赖 pm clear，彻底消除弹窗污染
 *
 * 背景（Round 3 根因）：
 *   clearData=true → pm clear 清除 consent.properties → App 重启弹「数据收集说明」
 *   globalSetup 只处理一次弹窗，各 spec 的 beforeEach 重启 App 后弹窗无人处理
 *
 * 方案 C 关键点：
 *   auth token 存放在 /data/data/${appId}/files/datastore/auth.preferences_pb
 *   consent 标记存放在 /data/data/${appId}/files/consent/consent.properties
 *   只删 auth.preferences_pb，保留 consent.properties（consent_mode=ALL）
 *   → App 重启 → 无 JWT → 跳转登录页 → 无弹窗
 */
import { execSync } from 'child_process';

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}

type ScreenState = 'login' | 'main' | 'room' | 'profile' | 'consent' | 'unknown';

/**
 * 通过 uiautomator dump 检测当前屏幕状态。
 */
async function detectScreenState(adbPrefix: string): Promise<ScreenState> {
  try {
    execSync(`${adbPrefix} shell uiautomator dump /sdcard/ui_detect_state.xml`, { stdio: 'pipe', timeout: 8000 });
    const xml = execSync(`${adbPrefix} shell cat /sdcard/ui_detect_state.xml`, { stdio: 'pipe', timeout: 5000 }).toString();

    // 同意弹窗（最高优先级检测）
    const consentMarkers = ['数据收集说明', '隐私政策', '用户协议', 'Privacy Policy', 'Data Collection'];
    if (consentMarkers.some(m => xml.includes(m))) return 'consent';

    // 登录页：包含手机号输入框或登录按钮（含阿拉伯语标记）
    const loginMarkers = ['手机号', '获取验证码', 'Send Code', 'phoneInput', 'login_btn', 'com.voice.room.android.local.debug:id/phone',
      'رقم الهاتف', 'إرسال رمز التحقق', 'رمز التحقق'];
    if (loginMarkers.some(m => xml.includes(m))) return 'login';

    // 主界面：包含底部 Tab 栏（首页/排行/钱包/我的等）
    const mainMarkers = ['tab_home', 'tab_rank', 'tab_wallet', 'tab_me', '首页', '排行榜', '我的', 'bottom_nav'];
    if (mainMarkers.some(m => xml.includes(m))) return 'main';

    // 房间页
    const roomMarkers = ['mic_seat', 'room_title', 'gift_btn', 'chat_input', 'mic_list'];
    if (roomMarkers.some(m => xml.includes(m))) return 'room';

    // 个人中心
    const profileMarkers = ['profile_avatar', 'sign_out', '退出登录', 'logout_btn'];
    if (profileMarkers.some(m => xml.includes(m))) return 'profile';

    return 'unknown';
  } catch {
    return 'unknown';
  }
}

/**
 * 通过 uiautomator dump 检测并关闭同意/权限弹窗。
 * 匹配按钮文本：同意/确定/Accept/Agree/OK/我已了解/知道了/Continue/全部同意
 *
 * Round 4 增强：maxAttempts 默认从 5 增加到 10，waitBetween 默认 1500ms，总超时最多 15s
 *
 * @param adbPrefix    ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 * @param maxAttempts  最多尝试关闭次数，默认 10（总超时 ~15s）
 * @param waitBetween  每次尝试间隔毫秒，默认 1500
 */
export async function dismissConsentDialog(
  adbPrefix: string,
  maxAttempts = 10,
  waitBetween = 1500
): Promise<void> {
  // Round 3 修复：dump 到 /sdcard 文件再 cat，避免 /dev/stdout pipe 截断问题
  const consentKeywords = ['同意', '确定', 'Accept', 'Agree', 'OK', '我已了解', '知道了', 'Continue', 'موافقت', 'قبول', '全部同意', '确认'];
  const nodeRegex = /text="([^"]*)"[^/]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/g;

  for (let i = 0; i < maxAttempts; i++) {
    try {
      execSync(`${adbPrefix} shell uiautomator dump /sdcard/ui_e2e_dismiss.xml`, { stdio: 'pipe', timeout: 8000 });
      const xml = execSync(`${adbPrefix} shell cat /sdcard/ui_e2e_dismiss.xml`, { stdio: 'pipe', timeout: 5000 }).toString();

      nodeRegex.lastIndex = 0;
      let dismissed = false;
      let match: RegExpExecArray | null;
      while ((match = nodeRegex.exec(xml)) !== null) {
        const text = match[1];
        if (consentKeywords.some((kw) => text.includes(kw))) {
          const cx = Math.floor((parseInt(match[2]) + parseInt(match[4])) / 2);
          const cy = Math.floor((parseInt(match[3]) + parseInt(match[5])) / 2);
          execSync(`${adbPrefix} shell input tap ${cx} ${cy}`, { stdio: 'pipe' });
          await sleep(waitBetween);
          dismissed = true;
          break;
        }
      }
      if (!dismissed) {
        break; // 没有弹窗，结束
      }
      await sleep(waitBetween);
    } catch {
      break;
    }
  }
}

/**
 * Round 4 方案 C：仅删除 JWT auth token（保留同意标记），重启 App 回到登录页。
 *
 * 优先路径（run-as 可用，debug build）：
 *   run-as ${appId} rm /data/data/${appId}/files/datastore/auth.preferences_pb
 *   → App 重启 → 无 JWT → 登录页 → 无弹窗（consent_mode=ALL 保留）
 *
 * 降级路径（run-as 不可用，release build）：
 *   pm clear → dismissConsentDialog 15s 强清弹窗
 *
 * @param appId  Android 应用包名，如 "com.voice.room.android.local.debug"
 * @param adbPrefix  ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 */
async function deleteAuthTokenOnly(adbPrefix: string, appId: string): Promise<'run-as-ok' | 'pm-clear-fallback'> {
  // 尝试 run-as 方式删除 auth.preferences_pb
  // DataStore 路径：/data/data/${appId}/files/datastore/auth.preferences_pb
  const authPbPath = `/data/data/${appId}/files/datastore/auth.preferences_pb`;
  const authPbTmpPath = `/data/data/${appId}/files/datastore/auth.preferences_pb.bak`;

  try {
    // 先验证 run-as 可用
    const idOut = execSync(`${adbPrefix} shell run-as ${appId} id 2>&1`, { stdio: 'pipe', timeout: 5000 }).toString();
    if (!idOut.includes('uid=')) throw new Error('run-as not working');

    // 删除 auth token 文件（备份到 .bak，降低 App 崩溃风险）
    execSync(`${adbPrefix} shell run-as ${appId} mv ${authPbPath} ${authPbTmpPath} 2>/dev/null || true`, { stdio: 'pipe', timeout: 5000 });
    // 同时尝试删除其他可能缓存 token 的文件（兼容不同 DataStore 版本）
    execSync(`${adbPrefix} shell run-as ${appId} find /data/data/${appId}/files/datastore/ -name "auth*.pb" -delete 2>/dev/null || true`, { stdio: 'pipe', timeout: 5000 });
    console.log(`[androidReset] ✅ run-as: deleted auth token at ${authPbPath}`);
    return 'run-as-ok';
  } catch {
    // 降级：pm clear（会清所有数据，包括同意标记，但随后用 dismissConsentDialog 处理）
    console.warn(`[androidReset] ⚠️ run-as failed, falling back to pm clear for ${appId}`);
    try {
      execSync(`${adbPrefix} shell pm clear ${appId}`, { stdio: 'pipe', timeout: 8000 });
    } catch { /* 忽略 */ }
    return 'pm-clear-fallback';
  }
}

/**
 * Round 4 方案 C：标准化 Android 测试前重置序列（不清数据，只删 JWT）
 *
 * 流程：
 *   1. force-stop（杀进程）
 *   2. run-as 删除 auth.preferences_pb（保留 consent.properties）
 *   3. am start 重启 App
 *   4. 等待 3s（App 初始化）
 *   5. dismissConsentDialog（安全网，正常情况下无弹窗出现）
 *   6. detectScreenState 验证：若非登录页则二次处理
 *   7. 最终等待 0.5s
 *
 * 向后兼容说明：
 *   - clearData 参数保留但语义变更：无论 true/false 均使用方案 C（只删 JWT）
 *   - 若 run-as 不可用，降级为 pm clear + 强化弹窗处理（15s）
 *   - 各 spec 无需修改调用签名
 *
 * @param adbPrefix        ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 * @param appId            Android 应用包名，如 "com.voice.room.android.local.debug"
 * @param maxDialogAttempts  关闭弹窗最大尝试次数，默认 5
 * @param clearData        Round 4 起：语义变更为"是否需要确保已登出"（不再 pm clear）
 *                         保留参数仅为向后兼容，内部统一走方案 C
 */
export async function resetAndroidToLoginPage(
  adbPrefix: string,
  appId: string,
  maxDialogAttempts = 5,
  clearData = false  // Round 4: 参数保留，但不再触发 pm clear
): Promise<void> {
  // Step 1: force-stop（杀进程，确保干净状态）
  try {
    execSync(`${adbPrefix} shell am force-stop ${appId}`, { stdio: 'pipe' });
  } catch { /* 忽略 */ }
  await sleep(800);

  // Step 2: 删除 auth token（方案 C 核心：只删 JWT，保留 consent 标记）
  // clearData=true 或 false 均走此路径（不再区分）
  const deleteMode = await deleteAuthTokenOnly(adbPrefix, appId);
  await sleep(300);

  // Step 3: am start — 使用经 adb resolve-activity 验证的 Activity 路径
  try {
    execSync(
      `${adbPrefix} shell am start --include-stopped-packages -n ${appId}/com.voice.room.android.presentation.MainActivity`,
      { stdio: 'pipe' }
    );
  } catch {
    // 降级：monkey 启动
    try {
      execSync(
        `${adbPrefix} shell monkey -p ${appId} -c android.intent.category.LAUNCHER 1`,
        { stdio: 'pipe' }
      );
    } catch { /* 忽略 */ }
  }

  // Step 4: 等待 App 初始化
  // pm-clear 降级路径需要更长等待（首次初始化）
  await sleep(deleteMode === 'pm-clear-fallback' ? 5000 : 3000);

  // Step 5: dismissConsentDialog（安全网）
  // run-as-ok 路径：consent 标记已保留，通常无弹窗，少量尝试即可
  // pm-clear 降级：consent 被清，需要更多重试（15s 强力关闭）
  const dialogAttempts = deleteMode === 'pm-clear-fallback' ? Math.max(maxDialogAttempts, 10) : maxDialogAttempts;
  await dismissConsentDialog(adbPrefix, dialogAttempts);
  await sleep(1000);

  // Step 6: 二次检测 — 验证是否在登录页
  const state = await detectScreenState(adbPrefix);
  if (state === 'consent') {
    // 仍有弹窗（罕见），再次强力关闭
    console.warn(`[androidReset] ⚠️ Consent dialog still present after dismiss, retrying...`);
    await dismissConsentDialog(adbPrefix, 10, 1000);
    await sleep(1000);
  } else if (state === 'main' || state === 'room' || state === 'profile') {
    // App 已登录（可能 auth.pb 删除未生效），强制二次删 token + 重启
    console.warn(`[androidReset] ⚠️ App still logged in (state: ${state}), retrying auth token deletion...`);
    try {
      execSync(`${adbPrefix} shell am force-stop ${appId}`, { stdio: 'pipe' });
    } catch { /* 忽略 */ }
    await sleep(500);
    await deleteAuthTokenOnly(adbPrefix, appId);
    await sleep(300);
    try {
      execSync(
        `${adbPrefix} shell am start --include-stopped-packages -n ${appId}/com.voice.room.android.presentation.MainActivity`,
        { stdio: 'pipe' }
      );
    } catch { /* 忽略 */ }
    await sleep(3500);
    await dismissConsentDialog(adbPrefix, dialogAttempts);
    await sleep(500);
  } else if (state === 'login') {
    console.log(`[androidReset] ✅ Login screen confirmed`);
  } else {
    console.log(`[androidReset] ℹ️ Screen state: ${state} (proceeding, login check deferred to agent)`);
  }

  await sleep(500);
}

// ─────────────────────────────────────────────────────────────────────────────
// Round 5: JWT 注入登录（方案 D）
// 绕过 UI 登录流，通过 API + DataStore proto 直接注入 JWT
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 将整数编码为 protobuf varint（小端序，每字节低 7 位有效，最高位为续位标志）
 */
function encodeVarint(n: number): Buffer {
  const bytes: number[] = [];
  while (n > 127) {
    bytes.push((n & 0x7f) | 0x80);
    n >>>= 7;
  }
  bytes.push(n & 0x7f);
  return Buffer.from(bytes);
}

/**
 * 将 JWT 字符串编码为 DataStore Preferences protobuf 二进制格式。
 *
 * 格式（逆向分析 auth.preferences_pb）：
 *   outer:  field 1 (tag=0x0a) + varint(entry_len) + entry_msg
 *   entry:  field 1 (0x0a) + len('jwt_token') + 'jwt_token'
 *           + field 2 (0x12) + varint(value_len) + value_msg
 *   value:  field 5 (0x2a) + varint(jwt_len) + jwt_bytes
 */
function buildDataStorePb(jwt: string): Buffer {
  const jwtBuf = Buffer.from(jwt, 'utf8');
  // value_msg: field 5 LEN = tag 0x2a + varint(jwt_len) + jwt_bytes
  const valueMsg = Buffer.concat([Buffer.from([0x2a]), encodeVarint(jwtBuf.length), jwtBuf]);
  // entry_msg: 0x0a + 0x09 + 'jwt_token' + 0x12 + varint(value_len) + value_msg
  const keyBuf = Buffer.from('jwt_token', 'utf8'); // 9 bytes
  const entryMsg = Buffer.concat([
    Buffer.from([0x0a, keyBuf.length]),
    keyBuf,
    Buffer.from([0x12]),
    encodeVarint(valueMsg.length),
    valueMsg,
  ]);
  // outer: 0x0a + varint(entry_len) + entry_msg
  return Buffer.concat([Buffer.from([0x0a]), encodeVarint(entryMsg.length), entryMsg]);
}

/**
 * Round 5 方案 D：通过 API 获取 JWT 并注入 DataStore，绕过 UI 登录流。
 *
 * 流程：
 *   1. 清除 Redis SMS cooldown（避免限速拒绝）
 *   2. 预置 OTP：HSET sms:code:{phone} code 123456
 *   3. POST /api/v1/auth/login → 获取 JWT
 *   4. 编码为 DataStore protobuf 格式
 *   5. adb run-as 写入 auth.preferences_pb
 */
async function loginViaJwtInjection(
  adbPrefix: string,
  appId: string,
  phone: string,
  serverUrl: string
): Promise<void> {
  // 1. 清除 SMS cooldown（忽略 docker 不可用情况）
  try {
    execSync(`docker exec vr-redis redis-cli DEL "sms:cooldown:${phone}"`, { stdio: 'pipe', timeout: 5000 });
  } catch { /* 忽略：redis 容器可能不可用 */ }

  // 2. 预置 OTP（直接 SET，绕过发送流程）
  try {
    execSync(`docker exec vr-redis redis-cli HSET "sms:code:${phone}" code 123456 attempts 0`, { stdio: 'pipe', timeout: 5000 });
  } catch { /* 忽略 */ }

  // 3. 调用登录 API 获取 JWT
  const loginOutput = execSync(
    `curl -s -X POST "${serverUrl}/api/v1/auth/login" ` +
    `-H "Content-Type: application/json" ` +
    `-d '{"phone":"${phone}","code":"123456"}'`,
    { stdio: 'pipe', timeout: 12000 }
  ).toString().trim();

  let jwt: string;
  try {
    const parsed = JSON.parse(loginOutput);
    // 响应格式：{"code":0,"message":"ok","data":{"token":"...","expires_in":...}}
    jwt = parsed?.data?.token || parsed?.token || parsed?.jwt || parsed?.access_token;
    if (!jwt || typeof jwt !== 'string') {
      throw new Error(`JWT not found in response: ${loginOutput.substring(0, 200)}`);
    }
  } catch (e) {
    throw new Error(`[androidReset] Login API failed for ${phone}: ${e}`);
  }

  // 4. 编码为 DataStore proto
  const pb = buildDataStorePb(jwt);
  const b64 = pb.toString('base64');

  // 5. 写入设备（两步法，避免 ADB run-as 双层 shell 引号剥离问题）
  //    Step A: 创建目录（直接传 mkdir 参数，无需 sh -c）
  try {
    execSync(
      `${adbPrefix} shell run-as ${appId} mkdir -p files/datastore`,
      { stdio: 'pipe', timeout: 5000 }
    );
  } catch { /* 目录可能已存在，忽略 */ }

  //    Step B: 写入文件（用外层单引号包裹整个 run-as 命令，确保 ADB 不剥离双引号）
  execSync(
    `${adbPrefix} shell 'run-as ${appId} sh -c "echo ${b64} | base64 -d > files/datastore/auth.preferences_pb"'`,
    { stdio: 'pipe', timeout: 12000 }
  );

  console.log(`[androidReset] ✅ JWT injected for ${phone} (${pb.length} bytes)`);
}

/**
 * Round 5 方案 D：完整重置 + JWT 注入，App 启动后直接进入主界面（跳过 UI 登录流）。
 *
 * 适用于：绝大多数需要"登录后"状态的测试用例。
 * 不适用于：TC-AUTH（专门测试登录 UI 流程）。
 *
 * 流程：
 *   1. force-stop App
 *   2. 通过 API 获取 JWT 并注入 auth.preferences_pb
 *   3. am start
 *   4. App 读取 JWT → 跳过登录页 → 直接进入主界面
 *   5. dismissConsentDialog 安全网（正常情况无弹窗）
 *
 * @param adbPrefix  ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 * @param appId      Android 应用包名，如 "com.voice.room.android.local.debug"
 * @param phone      手机号 E.164 格式，如 "+966500000900"
 * @param serverUrl  后端 API 地址（默认从 process.env.APP_SERVER_BASE_URL 读取）
 */
export async function resetAndroidToMainPage(
  adbPrefix: string,
  appId: string,
  phone = '+966500000900',
  serverUrl?: string
): Promise<void> {
  const apiBase = serverUrl ?? process.env.APP_SERVER_BASE_URL ?? 'http://192.168.1.19:3000';

  // Step 1: 回到 Home Screen（清除可能残留的系统 UI，如键盘设置页）
  try {
    execSync(`${adbPrefix} shell input keyevent KEYCODE_HOME`, { stdio: 'pipe' });
  } catch { /* 忽略 */ }
  await sleep(300);

  // Step 2: force-stop（杀进程，确保干净状态）
  try {
    execSync(`${adbPrefix} shell am force-stop ${appId}`, { stdio: 'pipe' });
  } catch { /* 忽略 */ }
  await sleep(300);

  // Step 3: JWT 注入（API login + write DataStore proto）
  await loginViaJwtInjection(adbPrefix, appId, phone, apiBase);
  await sleep(200);

  // Step 4: am start —— 触发 App 从 force-stop 状态启动，读取 JWT，自动跳转到主界面
  try {
    execSync(
      `${adbPrefix} shell am start --include-stopped-packages -n ${appId}/com.voice.room.android.presentation.MainActivity`,
      { stdio: 'pipe' }
    );
  } catch {
    try {
      execSync(
        `${adbPrefix} shell monkey -p ${appId} -c android.intent.category.LAUNCHER 1`,
        { stdio: 'pipe' }
      );
    } catch { /* 忽略 */ }
  }

  // Step 5: 等待 App 初始化（读取 JWT → 渲染主界面）
  await sleep(4000);

  // Step 6: 有针对性地处理同意弹窗——仅当 detectScreenState 检测到 'consent' 时才 dismiss
  // 目的：避免 dismissConsentDialog 误触主界面上与同意无关的"确定"/"确认"按钮导致导航异常
  const screenState = await detectScreenState(adbPrefix);
  if (screenState === 'consent') {
    console.log(`[androidReset] ⚠️ Consent dialog detected after JWT launch, dismissing...`);
    await dismissConsentDialog(adbPrefix, 8, 1200);
    await sleep(1000);
    // 确认弹窗已消除
    const stateAfter = await detectScreenState(adbPrefix);
    if (stateAfter === 'consent') {
      console.warn(`[androidReset] ⚠️ Consent dialog still present, one more round...`);
      await dismissConsentDialog(adbPrefix, 5, 1500);
      await sleep(800);
    }
  } else {
    console.log(`[androidReset] ✅ Screen state after launch: ${screenState} (no consent dialog)`);
  }

  console.log(`[androidReset] ✅ resetAndroidToMainPage done — ${phone} → main screen`);
}
