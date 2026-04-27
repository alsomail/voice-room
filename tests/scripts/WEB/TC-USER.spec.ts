/**
 * 测试套件：USER 用户管理（Web）
 * 用例来源：doc/tests/cases/WEB/TC-USER.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


async function login(page: any, user = 'admin_op', pw = 'Pass@123') {
  await page.goto('/login');
  const agent = new PlaywrightAgent(page);
  await agent.aiAction(`在用户名输入框输入 "${user}"`);
  await agent.aiAction(`在密码输入框输入 "${pw}"`);
  await agent.aiAction('点击"登录"按钮');
  await page.waitForURL(/dashboard/);
  return agent;
}

test.describe('TC-USER WEB - 用户管理', () => {
  test('TC-USER-00001: 列表 - 分页/搜索/角色权限', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/users');
    await agent.aiAction('在搜索框输入 "+966500" 并回车');
    await agent.aiAssert('表格仅显示手机号以 +966500 开头的用户');
    // 切换到 CS 账号验证权限
    await agent.aiAction('点击右上角用户头像，选择"退出登录"');
    await login(page, 'admin_cs', 'Pass@123');
    await page.goto('/users');
    await agent.aiAssert('列表正常可见，但"封禁"按钮置灰或不可见');
  });

  test('TC-USER-00002: 详情抽屉 + 封禁 E2E 多端闭环', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/users');
    await agent.aiAction('点击表格第一行用户昵称');
    await agent.aiAssert('右侧抽屉打开，显示用户详情：基本信息、钱包余额、最近 30 笔流水、绑定设备');
    await agent.aiAction('点击"封禁"按钮');
    await agent.aiAssert('弹出封禁弹窗，包含"临时/永久"、时长、原因字段');
    await agent.aiAction('选择"临时"，时长选择"24 小时"，原因输入"测试"，点击"确定"');
    await agent.aiAssert('抽屉内状态显示"已封禁，至 XXXX"，顶部出现成功提示');
  });

  test('TC-USER-00003: 解封弹窗 - 原因必填 + 二次确认', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/users');
    await agent.aiAction('通过筛选仅显示"已封禁"用户，点击第一行用户名');
    await agent.aiAction('在抽屉中点击"解封"按钮');
    await agent.aiAssert('弹出解封确认弹窗，原因输入框带红色 * 号必填标记');
    await agent.aiAction('不填原因直接点击"确定"');
    await agent.aiAssert('原因输入框下方出现红色错误文案"请填写解封原因"');
    await agent.aiAction('输入原因"申诉通过"，点击"确定"，再在二次确认中点击"确认解封"');
    await agent.aiAssert('抽屉状态刷新为"正常"，列表该行状态同步更新');
  });
});
