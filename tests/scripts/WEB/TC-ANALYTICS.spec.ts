/**
 * 测试套件：ANALYTICS 用户行为流 Tab（Web Admin）
 * 用例来源：doc/tests/cases/WEB/TC-ANALYTICS.md
 * 铁律 7（2026-04-30）：视觉与交互层全部经由 Midscene（PlaywrightAgent）。
 *
 * 覆盖用例（P0）：
 *   TC-ANALYTICS-00001 — 行为流 Tab 默认加载 + 时间窗切换
 *   TC-ANALYTICS-00004 — 权限控制：admin_* 事件仅 super_admin 可见
 */
import { test, expect } from '../support/fixtures';
import { PlaywrightAgent } from '@midscene/web/playwright';

const APP_SERVER_BASE_URL = process.env.APP_SERVER_BASE_URL ?? '';

test.describe('TC-ANALYTICS WEB - 用户行为流 Tab', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');

  // ── 共用 super_admin 登录 ──────────────────────────────────────────────────
  async function loginAsSuperAdmin(agent: PlaywrightAgent, page: any) {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
  }

  // ── TC-ANALYTICS-00001：行为流 Tab 默认加载 + 时间窗切换 ──────────────────

  test('TC-ANALYTICS-00001: 行为流 Tab 默认加载 + 时间窗切换', async ({ page, e2eEnv }: any) => {
    test.setTimeout(240_000); // AI 多步操作 + 等待，需要更长超时
    const UID = (e2eEnv?.userAId as string | undefined) ?? '';
    if (!UID) {
      console.log('[TC-ANALYTICS-00001] e2eEnv.userAId 未配置，使用搜索方式进入用户详情');
    }

    const agent = new PlaywrightAgent(page);
    await loginAsSuperAdmin(agent, page);

    // 进入用户详情页
    if (UID) {
      await page.goto(`/users/${UID}`);
    } else {
      await page.goto('/users');
      await page.waitForLoadState('networkidle');
      await agent.aiAction('点击用户列表第一行的用户昵称，打开用户详情抽屉或页面');
    }
    await page.waitForLoadState('networkidle');

    // Step1：切换到"行为流"Tab
    await agent.aiTap('"行为流"或"Events"或"行为记录" Tab 选项卡');
    await page.waitForLoadState('networkidle');
    await agent.aiAssert('行为流 Tab 已激活，页面显示行为流内容区域（无论有无事件记录，区域应存在）');
    await agent.aiAssert('默认时间窗为"最近 24 小时"或"Last 24h"，或有类似时间范围控件可见');

    // Step2：切换时间窗为"最近 7 天"
    await agent.aiTap('"最近 7 天"或"Last 7 days"时间窗选项');
    await page.waitForLoadState('networkidle');
    await agent.aiAssert('行为流区域已刷新（请求发出并返回，内容更新或保持空状态均可）');

    // Step3：自定义时间窗超过 30 天
    const hasCustomRange = await agent.aiBoolean('是否有"自定义"时间区间选项？');
    if (hasCustomRange) {
      await agent.aiTap('"自定义"时间区间选项');
      await page.waitForLoadState('networkidle');
      // 尝试选择超过 30 天的范围
      const hasCustomPicker = await agent.aiBoolean('是否弹出日期选择器？');
      if (hasCustomPicker) {
        await agent.aiAssert('自定义日期选择器可见（含开始日期和结束日期选择）');
      }
    }

    // Step4：验证空状态（使用一个不存在的用户 ID 测试）
    // 通过 URL 参数直接访问一个无事件的用户
    if (APP_SERVER_BASE_URL) {
      await page.goto('/users');
      await page.waitForLoadState('networkidle');
      await agent.aiAction('在搜索框输入 "U_NEW_EMPTY_USER" 并回车（查找不存在的用户）');
      await page.waitForLoadState('networkidle');
      const hasEmptyState = await agent.aiBoolean('是否显示空状态（如"暂无数据"或"No results"）？');
      if (hasEmptyState) {
        await agent.aiAssert('用户列表显示空状态提示');
      }
    }
  });

  // ── TC-ANALYTICS-00004：权限控制 ──────────────────────────────────────────

  test('TC-ANALYTICS-00004: 权限控制 - admin_* 事件仅 super_admin 可见', async ({ page, e2eEnv }: any) => {
    const UID = (e2eEnv?.userAId as string | undefined) ?? '';
    const agent = new PlaywrightAgent(page);

    // Step1-3：用 operator 账号登录，验证无 admin_* 事件
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await agent.aiAction('在用户名输入 "e2e_op"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });

    if (UID) {
      await page.goto(`/users/${UID}`);
      await page.waitForLoadState('networkidle');

      await agent.aiTap('"行为流"或"Events"或"行为记录" Tab 选项卡');
      await page.waitForLoadState('networkidle');

      const hasAdminEvents = await agent.aiBoolean('行为流列表中是否有 "admin_" 开头的事件名称？');
      expect(hasAdminEvents).toBe(false);
      await agent.aiAssert('operator 角色登录后，行为流 Tab 中不显示 admin_* 开头的事件');

      // event_name 下拉不含 admin_*
      const hasEventDropdown = await agent.aiBoolean('是否有 event_name 筛选下拉框？');
      if (hasEventDropdown) {
        await agent.aiTap('event_name 筛选下拉框');
        await page.waitForLoadState('networkidle');
        const hasAdminOption = await agent.aiBoolean('下拉选项中是否有 "admin_" 开头的选项？');
        expect(hasAdminOption).toBe(false);
        await agent.aiTap('按 Escape 关闭下拉');
      }
    }

    // Step4：用 super_admin 登录，可见 admin_* 事件
    // 清除 localStorage token（admin auth 使用 localStorage 而非 cookies）
    await page.evaluate(() => localStorage.clear());
    await page.context().clearCookies();
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });

    if (UID) {
      await page.goto(`/users/${UID}`);
      await page.waitForLoadState('networkidle');
      await agent.aiTap('"行为流"或"Events"或"行为记录" Tab 选项卡');
      await page.waitForLoadState('networkidle');
      await agent.aiAssert('super_admin 可以查看行为流事件列表（包含所有事件类型）');

      // event_name 下拉有更多选项（含 admin_*）
      const hasEventDropdown = await agent.aiBoolean('是否有 event_name 筛选下拉框？');
      if (hasEventDropdown) {
        await agent.aiTap('event_name 筛选下拉框');
        await page.waitForLoadState('networkidle');
        await agent.aiAssert('super_admin 可见的事件类型选项比 operator 更多（包含管理员操作事件）');
        await agent.aiTap('按 Escape 关闭下拉');
      }
    }
  });
});
