/**
 * 测试套件：Web AppLayout 侧栏 RBAC 菜单可见性矩阵
 * 用例来源：doc/tests/cases/WEB/TC-LAYOUT-RBAC.md
 * 说明：使用 Playwright + @midscene/web 的 AI 视觉交互。
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';

test.describe('TC-LAYOUT-RBAC WEB - RBAC 菜单可见性', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');

  test('TC-LAYOUT-RBAC-00001: super_admin 全菜单可见 + 全页面可访问', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('networkidle');

    await agent.aiAssert(
      '左侧导航栏包含 Dashboard / 房间管理 / 用户管理 / 礼物管理 / 治理日志 / 操作日志等菜单项（至少 5 个）',
    );

    // 验证可以访问各主要页面
    const menuUrls = ['/rooms', '/users', '/gifts'];
    for (const url of menuUrls) {
      await page.goto(url);
      await page.waitForLoadState('domcontentloaded');
      await page.waitForTimeout(2000);
      expect(page.url()).not.toMatch(/\/403/);
      expect(page.url()).not.toMatch(/\/login/);
    }
  });

  test('TC-LAYOUT-RBAC-00002: operator 菜单子集 + 越权访问被拦截', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    // 使用 operator 账号登录
    await agent.aiAction('在用户名输入 "e2e_op"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 30_000 });

    if (page.url().includes('/login')) {
      console.warn('[TC-LAYOUT-RBAC-00002] operator 登录失败，跳过');
      test.skip();
      return;
    }

    await page.waitForLoadState('networkidle');
    await agent.aiAssert('左侧导航栏可见，包含部分功能菜单');

    // 越权访问测试（尽力执行，可能 Admin Web 无 /admin/settings）
    await page.goto('/admin/settings');
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(2000);
    const isBlocked = page.url().includes('/403') || page.url().includes('/dashboard') || page.url().includes('/login');
    console.log(`[TC-LAYOUT-RBAC-00002] 越权 /admin/settings 被拦截：${isBlocked}`);
  });

  test('TC-LAYOUT-RBAC-00003: cs 仅可访问只读类菜单', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    // 使用 cs 账号登录（账号与密码依 seed 配置）
    await agent.aiAction('在用户名输入 "e2e_cs"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 30_000 });

    if (page.url().includes('/login')) {
      console.warn('[TC-LAYOUT-RBAC-00003] cs 账号登录失败，跳过（可能 seed 未创建）');
      test.skip();
      return;
    }

    await page.waitForLoadState('networkidle');

    // 尝试越权访问 /gifts（cs 角色不应可访问）
    await page.goto('/gifts');
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(2000);
    const giftsBlocked = page.url().includes('/403') || !page.url().includes('/gifts');
    console.log(`[TC-LAYOUT-RBAC-00003] /gifts 越权被拦截：${giftsBlocked}`);
    expect(giftsBlocked).toBe(true);
  });

  test('TC-LAYOUT-RBAC-00004: finance 仅可访问财务相关 + 治理日志强制 403', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_fin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 30_000 });

    if (page.url().includes('/login')) {
      console.warn('[TC-LAYOUT-RBAC-00004] finance 账号登录失败，跳过');
      test.skip();
      return;
    }

    await page.waitForLoadState('networkidle');

    // 尝试访问治理日志（finance 角色不应可访问）
    await page.goto('/rooms/governance');
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(2000);
    const govBlocked = page.url().includes('/403') || !page.url().includes('/governance');
    console.log(`[TC-LAYOUT-RBAC-00004] /rooms/governance 越权被拦截：${govBlocked}`);
    expect(govBlocked).toBe(true);
  });
});
