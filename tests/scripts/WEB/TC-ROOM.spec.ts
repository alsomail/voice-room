/**
 * 测试套件：ROOM 房间监控（Web）
 * 用例来源：doc/tests/cases/WEB/TC-ROOM.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';
import 'dotenv/config';

const URL = process.env.ADMIN_WEB_URL ?? 'http://localhost:5173';

async function login(page: any) {
  await page.goto(`${URL}/login`);
  const agent = new PlaywrightAgent(page);
  await agent.aiAction('在用户名输入框输入 "admin_op"');
  await agent.aiAction('在密码输入框输入 "Pass@123"');
  await agent.aiAction('点击"登录"按钮');
  await page.waitForURL(/dashboard/);
  return agent;
}

test.describe('TC-ROOM WEB - 房间监控', () => {
  test('TC-ROOM-00001: Dashboard 概览 + ECharts + 30s 自动刷新', async ({ page }) => {
    const agent = await login(page);
    await page.goto(`${URL}/dashboard`);
    await agent.aiAssert('页面顶部有 4 个数字卡片：在线用户、活跃房间、今日新增用户、今日打赏总额；下方有两个 ECharts 图表');
    // 等待 30s 刷新
    await page.waitForTimeout(31_000);
    await agent.aiAssert('卡片数据或时间戳有更新变化，未出现报错提示');
  });

  test('TC-ROOM-00002: 房间列表 - 搜索 / 筛选 / 分页', async ({ page }) => {
    const agent = await login(page);
    await page.goto(`${URL}/rooms`);
    await agent.aiAction('在搜索框输入 "test" 并按回车');
    await agent.aiAssert('表格仅显示标题或房主包含 test 的房间');
    await agent.aiAction('在状态筛选下拉选择"已关闭"');
    await agent.aiAssert('所有行状态列显示"已关闭"');
    await agent.aiAction('点击分页控件切换到第 2 页');
    await agent.aiAssert('表格内容刷新为另一批数据');
  });

  test('TC-ROOM-00003: 详情抽屉 - 强制关闭完整闭环', async ({ page }) => {
    const agent = await login(page);
    await page.goto(`${URL}/rooms`);
    await agent.aiAction('点击表格第一行的房间标题');
    await agent.aiAssert('页面右侧滑出抽屉，显示房间详情：房主、创建时间、在线人数、聊天记录、礼物记录');
    await agent.aiAction('点击抽屉中"强制关闭房间"按钮');
    await agent.aiAssert('弹出二次确认对话框，要求输入关闭原因');
    await agent.aiAction('在原因输入框输入"测试关闭"，点击确定');
    await agent.aiAssert('抽屉内状态变为"已关闭"，顶部出现绿色成功提示');
  });

  test('TC-ROOM-00004: XSS 防护 - 标题恶意输入', async ({ page }) => {
    const agent = await login(page);
    await page.goto(`${URL}/rooms`);
    await agent.aiAction('在搜索框输入 "<script>alert(1)</script>" 并回车');
    // 不应弹出浏览器原生 alert
    page.on('dialog', (d) => { throw new Error('XSS alert leaked: ' + d.message); });
    await agent.aiAssert('页面正常渲染，表格为空或显示"无匹配房间"，字符以纯文本形式呈现');
  });

  test('TC-ROOM-00005: 活跃房间监控增强 - 状态/时长/异常高亮', async ({ page }) => {
    const agent = await login(page);
    await page.goto(`${URL}/rooms/active`);
    await agent.aiAssert('表格新增"状态"（进行中/异常）、"持续时长"列；异常状态行以红色背景高亮');
    await agent.aiAction('点击"仅显示异常"筛选按钮');
    await agent.aiAssert('表格仅剩红色高亮行');
  });
});
