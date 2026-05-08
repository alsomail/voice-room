/**
 * androidReset.ts — 标准化 Android 测试前重置序列
 *
 * Round 3 P0 修复：消除 pm clear 触发的同意弹窗污染 + 顺序污染
 *
 * 核心策略：
 *   - 使用 force-stop → am start（不 pm clear），避免弹窗
 *   - 通过 uiautomator dump 主动关闭同意弹窗（最多 N 次）
 *   - 不依赖 Midscene agent，纯 ADB 实现，可在 agent.launch() 前调用
 */
import { execSync } from 'child_process';

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}

/**
 * 通过 uiautomator dump 检测并关闭同意/权限弹窗。
 * 匹配按钮文本：同意/确定/Accept/Agree/OK/我已了解/知道了/Continue
 *
 * @param adbPrefix  ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 * @param maxAttempts  最多尝试关闭次数，默认 5
 */
export async function dismissConsentDialog(adbPrefix: string, maxAttempts = 5): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const xml = execSync(
        `${adbPrefix} shell uiautomator dump /dev/stdout 2>/dev/null`,
        { stdio: ['pipe', 'pipe', 'pipe'] }
      ).toString();

      // 匹配同意/确定/Accept/Agree/OK/我已了解/知道了/Continue 按钮
      const btnMatch = xml.match(
        /text="([^"]*(?:同意|确定|Accept|Agree|OK|我已了解|知道了|Continue)[^"]*)"\s[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/i
      );
      if (!btnMatch) {
        break; // 没有弹窗，结束
      }
      const cx = Math.floor((parseInt(btnMatch[2]) + parseInt(btnMatch[4])) / 2);
      const cy = Math.floor((parseInt(btnMatch[3]) + parseInt(btnMatch[5])) / 2);
      execSync(`${adbPrefix} shell input tap ${cx} ${cy}`, { stdio: 'pipe' });
      await sleep(1000);
    } catch {
      break;
    }
  }
}

/**
 * 标准化 Android 测试前重置序列。
 *
 * 流程：force-stop → am start（不 pm clear，避免弹窗）→ 等待 3s → 关闭弹窗 × maxDialogAttempts → 等待 0.5s
 *
 * 调用方应在此函数返回后立刻调用 agent.launch(appId) 以让 Midscene 接管。
 *
 * @param adbPrefix        ADB 命令前缀，如 "adb -s 9A251FFAZ00EAJ"
 * @param appId            Android 应用包名，如 "com.voice.room.android.local.debug"
 * @param maxDialogAttempts  关闭弹窗最大尝试次数，默认 5
 */
export async function resetAndroidToLoginPage(
  adbPrefix: string,
  appId: string,
  maxDialogAttempts = 5
): Promise<void> {
  // 1. force-stop（不 pm clear 避免弹窗）
  try {
    execSync(`${adbPrefix} shell am force-stop ${appId}`, { stdio: 'pipe' });
  } catch { /* 忽略 */ }
  await sleep(800);

  // 2. am start — 使用正确的 Activity 路径（经 adb resolve-activity 验证）
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
  await sleep(3000);

  // 3. 关闭弹窗（最多 maxDialogAttempts 次）
  await dismissConsentDialog(adbPrefix, maxDialogAttempts);
  await sleep(500);
}
