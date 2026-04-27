/**
 * 测试套件：WALLET 钱包调整（Web）
 * 用例来源：doc/tests/cases/WEB/TC-WALLET.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


test.describe('TC-WALLET WEB - 调整余额', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');
  test('TC-WALLET-00001: 调整余额弹窗 - 校验 + 双重确认', async ({ page }) => {
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入框输入 "e2e_fin"');
    await agent.aiAction('在密码输入框输入 "admin_password_change_me"');
    await agent.aiAction('点击"登录"按钮');
    await page.waitForURL(/dashboard/);

    await page.goto('/users');
    await agent.aiAction('点击表格第一行用户昵称，然后在详情抽屉中点击"调整余额"按钮');
    await agent.aiAssert('弹出调整余额 Modal，包含"当前余额"展示、"变动值（正负）"输入、"原因"输入、"二次确认"复选框');

    // 非数值
    await agent.aiAction('在变动值输入框输入 "abc"，点击确定');
    await agent.aiAssert('变动值输入框下方提示"请输入整数"');

    // 未勾选二次确认
    await agent.aiAction('清空变动值，输入 "-100"，原因输入"纠正"，不勾选二次确认，点击"确定"');
    await agent.aiAssert('"确定"按钮保持置灰状态，或出现提示"请确认"');

    // 正常提交
    await agent.aiAction('勾选"二次确认"复选框，然后点击"确定"');
    await agent.aiAssert('Modal 关闭，顶部出现绿色成功提示，抽屉中的余额数字刷新');
  });
});
