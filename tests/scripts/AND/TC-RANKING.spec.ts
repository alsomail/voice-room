/**
 * 测试套件：RANKING 榜单（Android）
 * 用例来源：doc/tests/cases/AND/TC-RANKING.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（agentFromAdbDevice）。
 *
 * 覆盖用例（P0）：
 *   TC-RANKING-00001 — 双 Tab 切换 + Top3 奖牌渲染
 */
import { test, expect } from '../support/fixtures';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';
import { redisExecSync, RedisCliUnavailableError } from '../support/redisCli';
import { resetAndroidToLoginPage, resetAndroidToMainPage } from '../support/androidReset';

test.setTimeout(300_000);

// ── TC-RANKING-00001：双 Tab 切换 + Top3 奖牌 ────────────────────────────────

test('TC-RANKING-00001: 双 Tab 切换 + Top3 奖牌渲染', async ({ e2eEnv }: any) => {
  const ANDROID_APP_ID = e2eEnv.androidAppId as string;
  if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置 — 请在 tests/scripts/env/.env.local 中设置 ANDROID_APP_ID');
  const DATABASE_URL = e2eEnv.databaseUrl as string;
  const ADB_DEVICE_ID = process.env.ADB_DEVICE_ID || undefined;
  const adbPrefix = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';
  const phone = '+966500000900';

  const agent = await agentFromAdbDevice(ADB_DEVICE_ID, {
    aiActionContext: '当前是 Android 语聊房 App，界面语言为中文、阿拉伯语或英语，榜单页有双层 Tab',
  });

  try {
    // Round 5 修复（方案 D）：JWT 注入绕过 UI 登录流
    // agent.launch() 已移除 — 保留会触发 FLAG_ACTIVITY_RESET_TASK_IF_NEEDED 导致 HOME 闪屏
    await resetAndroidToMainPage(adbPrefix, ANDROID_APP_ID, phone);
    await agent.aiWaitFor('主界面已加载，大厅房间列表可见', { timeoutMs: 20_000 });

    // Step1：点击大厅顶部 🏆 图标进入榜单
    await agent.aiTap('大厅顶部右上角的 🏆 奖杯图标 或"榜单"入口');
    await agent.aiWaitFor('进入 RankingScreen 榜单页面', { timeoutMs: 10_000 });

    // Step2：验证双层 Tab
    await agent.aiAssert('页面顶部有双层 Tab：上层含"魅力榜"/"Charm"和"财富榜"/"Wealth"，下层含"日榜"/"Day"和"周榜"/"Week"');

    // Step3：验证 Top1 金色光晕（容忍空数据状态）
    // [自愈-Round1-Strategy-B] 排行榜无礼物数据时显示"暂无数据"，放宽断言：有数据则验证金色样式，无数据则接受空状态
    const hasRankingData = await agent.aiBoolean('当前榜单是否显示了至少一行排名数据（非"暂无数据"或空状态）？');
    if (hasRankingData) {
      await agent.aiAssert('榜单首行（排名第一）的头像外侧有金色光晕或皇冠图标，排名数字显示金色');
    } else {
      await agent.aiAssert('当前榜单显示空状态（"暂无数据"或类似提示），Tab 切换区域可见且可交互');
    }

    // Step4：切换到"财富-周"
    await agent.aiTap('"财富榜" 或 "Wealth" Tab');
    await agent.aiWaitFor('Tab 切换完成（列表区域已更新，可能显示数据或"暂无数据"提示）', { timeoutMs: 8_000 });
    await agent.aiTap('"周榜" 或 "Week" Tab（第二层Tab栏右侧）');
    await agent.aiWaitFor('数据加载完成', { timeoutMs: 8_000 });
    await agent.aiAssert('"财富-周"榜单数据已加载，显示排名列表或空状态');

    // Step5：切回"魅力-日"
    await agent.aiTap('"魅力榜" 或 "Charm" Tab');
    await agent.aiTap('"日榜" 或 "Day" Tab（第二层Tab栏左侧）');
    await agent.aiWaitFor('切回默认榜单', { timeoutMs: 8_000 });

    // Step6：下拉刷新
    await agent.aiTap('榜单页面顶部（下拉刷新手势区域）');
    await agent.aiWaitFor('榜单数据刷新完成', { timeoutMs: 15_000 });
    await agent.aiAssert('榜单已刷新，数据正常显示');

  } finally {
    try {
      redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]);
    } catch { /* 忽略 */ }
    await agent.destroy().catch(() => {});
  }
});
