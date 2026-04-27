/**
 * 测试套件：GIFT 礼物管理（Web）
 * 用例来源：doc/tests/cases/WEB/TC-GIFT.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


async function login(page: any) {
  await page.goto('/login');
  const agent = new PlaywrightAgent(page);
  await agent.aiAction('在用户名输入框输入 "e2e_op"');
  await agent.aiAction('在密码输入框输入 "admin_password_change_me"');
  await agent.aiAction('点击"登录"按钮');
  await page.waitForURL(/dashboard/);
  return agent;
}

test.describe('TC-GIFT WEB - 礼物管理', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');
  test('TC-GIFT-00001: 列表 + 筛选', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/gifts');
    await agent.aiAssert('页面标题为"礼物管理"，顶部有"新增礼物"按钮，表格包含 ID/中文名/阿语名/价格/排序/状态 列');
    await agent.aiAction('在筛选区选择状态为"已下架"并点击查询');
    await agent.aiAssert('表格中所有行的状态列都显示"已下架"或灰色标签');
  });

  test('TC-GIFT-00002: 新增礼物 + 图片白名单 + CRUD', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/gifts');
    await agent.aiAction('点击右上角"新增礼物"按钮');
    await agent.aiAssert('弹出新增礼物的 Modal，包含 ID/名称(中)/名称(阿)/价格/排序/图片上传 等字段');

    // 非法后缀
    await page.setInputFiles('input[type=file]', {
      name: 'x.exe', mimeType: 'application/x-msdownload', buffer: Buffer.from('MZ'),
    });
    await agent.aiAssert('弹出红色错误提示，内容包含"仅支持 png/jpg/webp"');

    // 合法图片 + 创建
    await page.setInputFiles('input[type=file]', {
      name: 'rose.png', mimeType: 'image/png', buffer: Buffer.from([0x89, 0x50, 0x4e, 0x47]),
    });
    await agent.aiAction('在 ID 输入框输入 "test_' + Date.now() + '"');
    await agent.aiAction('在中文名称输入"测试"，阿语名称输入"اختبار"，价格输入 5，排序输入 99');
    await agent.aiAction('点击 Modal 底部的"保存"按钮');
    await agent.aiAssert('页面顶部出现绿色成功提示"新增成功"，列表中多出新礼物行');

    // 编辑
    await agent.aiAction('在刚才创建的礼物所在行点击"编辑"按钮');
    await agent.aiAction('将价格改为 8 并点击"保存"');
    await agent.aiAssert('列表中该行价格列显示为 8');

    // 下架
    await agent.aiAction('在同一行点击"下架"按钮并在确认框中点击"确定"');
    await agent.aiAssert('该行状态列变为"已下架"');
  });
});
