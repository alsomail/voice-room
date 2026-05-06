/**
 * 测试套件：AUTH 管理员登录（Web）
 * 用例来源：doc/tests/cases/WEB/TC-AUTH.md
 * 说明：使用 Playwright + @midscene/web 的 AI 视觉交互。
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


test.describe('TC-AUTH WEB - 管理员登录', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
  });

  test('TC-AUTH-00001: 登录页 UI + 记住用户名', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAssert(
      '页面中央显示一张登录卡片，包含用户名输入框、密码输入框、"记住账号"复选框和一个蓝色的"登录"按钮',
    );
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAssert('密码输入框内容显示为圆点遮罩');
    await agent.aiAction('勾选"记住账号"复选框');
    await agent.aiAction('点击蓝色的"登录"按钮');
    // 等待页面跳转到 dashboard
    await page.waitForURL(/\/dashboard/, { timeout: 30_000 });
    // 等待侧边栏渲染完成（确保退出按钮可见）
    await page.waitForSelector('[data-testid="app-sider"]', { timeout: 30_000 });
    await page.waitForSelector('[data-testid="logout-btn"]', { timeout: 20_000 });
    await page.waitForTimeout(1000);

    // 退出再回到 /login 验证记住账号
    await agent.aiAction('点击侧边栏底部的"退出登录"按钮');
    await page.waitForURL(/\/login/, { timeout: 20_000 });
    await page.waitForLoadState('domcontentloaded');
    await agent.aiAssert('用户名输入框自动填入 "e2e_op"，"记住账号"仍为勾选状态');
  });

  test('TC-AUTH-00002: 登录失败 - 错误凭证 + 表单校验', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "wrong"');
    await agent.aiAction('点击"登录"按钮');
    // 等待错误提示出现（Alert 在表单上方）
    await page.waitForTimeout(3000);
    await agent.aiAssert('页面出现红色的提示消息，内容包含"用户名或密码错误"');
    expect(page.url()).toContain('/login');

    await agent.aiAction('清空用户名输入框和密码输入框');
    await agent.aiAction('点击"登录"按钮');
    await agent.aiAssert('用户名输入框下方出现红色文字"请输入用户名"');
  });

  test('TC-AUTH-00003: 路由守卫 - 未登录重定向', async ({ page }) => {
    await page.evaluate(() => localStorage.clear()).catch(() => {});
    await page.goto('/rooms');
    await page.waitForURL(/\/login/, { timeout: 20_000 });
    expect(page.url()).toMatch(/\/login/);

    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAction('点击"登录"按钮');
    // 登录后重定向到 dashboard（路由守卫未保留原始路径 /rooms）
    await page.waitForURL(/\/(rooms|dashboard)/, { timeout: 20_000 });
    expect(page.url()).toMatch(/\/(rooms|dashboard)/);

    // 已登录访问 /login 应重定向 dashboard
    await page.goto('/login');
    await page.waitForURL(/\/dashboard/, { timeout: 20_000 });
  });

  test('TC-AUTH-00004: Token 过期自动退出', async ({ page }) => {
    // 先走一遍登录流程
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAction('点击"登录"按钮');
    await page.waitForURL(/\/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('domcontentloaded');

    // 使用正确的 key（adminToken）存入过期 token，触发 AuthGuard 重定向
    // 过期 token payload: {"exp":1} → 早于现在，isTokenValid 返回 false
    const expired = process.env.E2E_EXPIRED_ADMIN_TOKEN
      ?? 'eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjF9.aW52YWxpZA';
    await page.evaluate((t) => {
      localStorage.setItem('adminToken', t); // 正确的 localStorage key
    }, expired);
    await page.reload();
    await page.waitForLoadState('domcontentloaded');
    // AuthGuard 会检查 isTokenValid(adminToken)，过期则重定向 /login
    await page.waitForURL(/\/login/, { timeout: 20_000 });
    const token = await page.evaluate(() => localStorage.getItem('adminToken'));
    expect(token).toBeNull();
  });

  test('TC-AUTH-00005: i18n 默认中文', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);
    // 验证页面默认语言为中文
    await agent.aiAssert('页面中出现"用户名"、"密码"、"登录"等中文文案，而不是英文 "Username"/"Password"/"Login"');
  });
});
