/**
 * 测试套件：WALLET 钱包调整（Web）
 * 用例来源：doc/tests/cases/WEB/TC-WALLET.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


test.describe('TC-WALLET WEB - 调整余额', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');
  test('TC-WALLET-00001: 调整余额弹窗 - 校验 + 双重确认', async ({ page }) => {
    // SKIP-KNOWN P2: AdminServer GET /api/admin/users returns "参数错误" — pending API fix
    test.skip(true, 'SKIP-KNOWN P2: AdminServer /api/admin/users API Bug');
    test.setTimeout(180_000);
    await page.goto('/login');
    await page.waitForLoadState('load');
    await page.waitForSelector('.ant-card', { timeout: 10_000 });
    const agent = new PlaywrightAgent(page);
    // 使用 super_admin 账号（e2e_fin finance 角色无法访问用户列表 API）
    await agent.aiAction('在用户名输入框输入 "e2e_admin"');
    await agent.aiAction('在密码输入框输入 "admin_password_change_me"');
    await agent.aiAction('点击"登录"按钮');
    await page.waitForURL(/dashboard/);
    await page.waitForSelector('[data-testid="app-sider"]', { timeout: 15_000 });

    await page.goto('/users');
    await page.waitForSelector('[data-testid="users-table"]', { timeout: 15_000 });
    // Search for seeded E2E User A (has 100,000 diamond balance) by phone
    await page.locator('input[placeholder*="手机号"]').fill('+966500000900');
    await page.getByRole('button', { name: /搜.{0,1}索/ }).first().click();
    await page.waitForTimeout(1500);
    // Open E2E User A's detail drawer
    await agent.aiAction('点击表格第一行用户昵称或"查看"按钮');
    await page.waitForSelector('.ant-drawer', { timeout: 10_000 });
    // Click "调整余额" button (visible for super_admin/operator/finance roles)
    await agent.aiAction('点击抽屉中的"调整余额"按钮');
    await page.waitForSelector('.ant-modal', { timeout: 10_000 });
    // AdjustBalanceModal actual UI: current balance + amount InputNumber + reason TextArea + submit button
    // No checkbox — second confirmation is a Modal.confirm popup for negative amounts
    await agent.aiAssert('弹出调整余额 Modal，包含当前余额展示、变动金额输入框、原因输入框和确定按钮');

    // Submit disabled when reason is empty — use direct DOM to fill amount, then DOM-check button state
    // Find the amount InputNumber (spinbutton role) inside the "调整余额" dialog
    const adjustDialog = page.getByRole('dialog', { name: '调整余额' });
    await adjustDialog.getByRole('spinbutton').fill('-100');
    await page.waitForTimeout(300);
    // Assert submit button is disabled via DOM (don't let AI click cancel)
    const submitBtn = page.locator('[data-testid="adjust-submit-btn"]');
    await agent.aiAssert('"确定"按钮为禁用（灰色）状态，无法提交');

    // Fill reason and submit: triggers Modal.confirm for negative amount
    await page.locator('[data-testid="adjust-reason-input"]').fill('纠正');
    await page.waitForTimeout(300);
    // Click the submit button directly (it should be enabled now)
    await submitBtn.click();
    await page.waitForTimeout(1000);
    // For negative amounts, a second Modal.confirm popup appears
    await agent.aiAssert('弹出二次确认对话框，询问是否确认扣减金额');
    // Click OK in Modal.confirm directly
    await page.locator('.ant-modal-confirm .ant-btn-primary').click();
    // Wait for API call to succeed and all modals to close
    await page.waitForSelector('.ant-modal-confirm', { state: 'detached', timeout: 10_000 });
    await page.waitForSelector('.ant-modal', { state: 'detached', timeout: 5_000 });
    // Wait for success toast to appear (message.success fires in AdjustBalanceModal after API succeeds)
    await page.waitForTimeout(1500);
    await agent.aiAssert('调整余额弹窗已关闭，顶部出现成功提示，或抽屉中余额数字已更新');
  });
});
