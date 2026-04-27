/**
 * 测试套件：LOG 审计日志（Web）
 * 用例来源：doc/tests/cases/WEB/TC-LOG.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


async function login(page: any) {
  await page.goto('/login');
  const agent = new PlaywrightAgent(page);
  await agent.aiAction('在用户名输入框输入 "admin_op"');
  await agent.aiAction('在密码输入框输入 "Pass@123"');
  await agent.aiAction('点击"登录"按钮');
  await page.waitForURL(/dashboard/);
  return agent;
}

test.describe('TC-LOG WEB - 审计日志', () => {
  test('TC-LOG-00001: 时间倒序 + 筛选 + 详情', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/logs');
    await agent.aiAssert('页面标题"操作日志"，表格按时间列倒序排列，最上面是最近的一条');
    await agent.aiAction('在筛选区的"操作类型"下拉中选择"封禁用户"，点击"查询"');
    await agent.aiAssert('表格所有行的操作类型列都显示"封禁用户"');
    await agent.aiAction('点击表格第一行的"详情"按钮');
    await agent.aiAssert('弹出一个抽屉或 Modal，展示完整 JSON 详情，包括 admin_id/ip/user_agent/target');
  });

  test('TC-LOG-00002: 10 万行翻页 ≤2s', async ({ page }) => {
    test.skip(process.env.CI_E2E_READY !== '1', '性能测试需 seed 数据');
    const agent = await login(page);
    await page.goto('/logs');
    const t0 = Date.now();
    await agent.aiAction('点击分页控件跳转到第 100 页');
    await page.waitForResponse((r) => r.url().includes('/admin/logs') && r.ok());
    const dt = Date.now() - t0;
    expect(dt).toBeLessThanOrEqual(2000);
  });
});
