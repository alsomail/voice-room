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
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    const agent = new PlaywrightAgent(page);

    await agent.aiAssert(
      '页面中央显示一张登录卡片，顶部为“语聊房管理后台”Logo，包含用户名输入框、密码输入框、“记住账号”复选框和一个蓝色的“登录”按钮',
    );
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAssert('密码输入框内容显示为圆点遮罩');
    await agent.aiAction('勾选“记住账号”复选框');
    await agent.aiAction('点击蓝色的“登录”按钮');
    await agent.aiAssert('登录按钮上出现圆形 Loading 动画');
    await page.waitForURL(/\/dashboard/, { timeout: 10_000 });
    expect(page.url()).toContain('/dashboard');

    // 退出再回到 /login 验证记住账号
    await agent.aiAction('点击右上角的用户头像或菜单，选择“退出登录”');
    await page.waitForURL(/\/login/, { timeout: 10_000 });
    await agent.aiAssert('用户名输入框自动填入 "e2e_op"，密码输入框为空，“记住账号”仍为勾选状态');
  });

  test('TC-AUTH-00002: 登录失败 - 错误凭证 + 表单校验', async ({ page }) => {
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "wrong"');
    await agent.aiAction('点击“登录”按钮');
    await agent.aiAssert('页面顶部弹出红色的提示消息，内容包含“用户名或密码错误”');
    expect(page.url()).toContain('/login');

    await agent.aiAction('清空用户名输入框和密码输入框');
    await agent.aiAction('点击“登录”按钮');
    await agent.aiAssert('用户名输入框下方出现红色文字“请输入用户名”');
  });

  test('TC-AUTH-00003: 路由守卫 - 未登录重定向', async ({ page }) => {
    await page.evaluate(() => localStorage.clear()).catch(() => {});
    await page.goto('/rooms');
    await page.waitForURL(/\/login\?redirect=/);
    expect(page.url()).toMatch(/\/login\?redirect=.*rooms/);

    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAction('点击“登录”按钮');
    await page.waitForURL(/\/rooms/, { timeout: 10_000 });

    // 已登录访问 /login 应重定向 dashboard
    await page.goto('/login');
    await page.waitForURL(/\/dashboard/);
  });

  test('TC-AUTH-00004: Token 过期自动退出', async ({ page }) => {
    // 先走一遍登录流程
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入框中输入 "e2e_op"');
    await agent.aiAction('在密码输入框中输入 "admin_password_change_me"');
    await agent.aiAction('点击“登录”按钮');
    await page.waitForURL(/\/dashboard/);

    // 替换 token 为已知过期 token
    const expired = process.env.E2E_EXPIRED_ADMIN_TOKEN ?? 'expired.token.value';
    await page.evaluate((t) => localStorage.setItem('admin_token', t), expired);
    await page.reload();
    await agent.aiAssert('页面顶部弹出提示消息“登录已过期，请重新登录”');
    await page.waitForURL(/\/login/, { timeout: 5_000 });
    const token = await page.evaluate(() => localStorage.getItem('admin_token'));
    expect(token).toBeNull();
  });

  test('TC-AUTH-00005: i18n 中英切换 + 持久化', async ({ page }) => {
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('点击右上角的“语言”下拉菜单，选择 “English”');
    await agent.aiAssert('页面中出现 "Username"、"Password"、"Login" 英文文案');
    await agent.aiAction('再次点击语言下拉菜单，选择 “简体中文”');
    await agent.aiAssert('页面文案恢复为中文“用户名”、“密码”、“登录”');
    await page.reload();
    await agent.aiAssert('刷新后页面仍显示中文');
  });
});
