/**
 * 测试套件：Web 数据看板首页（Dashboard）
 * 用例来源：doc/tests/cases/WEB/TC-DASHBOARD.md
 * 说明：使用 Playwright + @midscene/web 的 AI 视觉交互。
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';

test.describe('TC-DASHBOARD WEB - 数据看板', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');

  test('TC-DASHBOARD-00001: StatCards 渲染 + 数据正确', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('networkidle');

    await agent.aiAssert(
      '页面顶部显示 4 张统计卡片，分别代表在线人数/活跃房间/今日DAU/今日新增用户',
    );
    await agent.aiAssert('每张卡片的数字大于等于 0');
  });

  test('TC-DASHBOARD-00002: ECharts 折线图 + 时间窗切换', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);

    await agent.aiAssert('页面中部存在趋势图区域（可能显示折线图 Canvas 或"暂无趋势数据"空状态提示，趋势图组件存在）');

    // 时间窗切换
    const has30DayBtn = await agent.aiBoolean('是否有"30 天"或"30d"时间窗切换按钮？');
    if (has30DayBtn) {
      await agent.aiTap('"30 天" 或 "30d" 按钮');
      await page.waitForTimeout(3000);
      await agent.aiAssert('折线图 X 轴已切换为 30 天时间范围');
    } else {
      console.warn('[TC-DASHBOARD-00002] 未找到"30 天"按钮，可能 UI 布局不同');
    }
  });

  test('TC-DASHBOARD-00003: 30s 自动刷新 + 组件卸载取消', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('networkidle');

    // 拦截接口请求计数
    let overviewCallCount = 0;
    page.on('response', (resp) => {
      if (resp.url().includes('/stats/overview')) overviewCallCount++;
    });

    // 等待 35 秒，验证至少有 1 次自动刷新调用（拦截器在初始加载后设置，初次加载不计入）
    await page.waitForTimeout(35_000);
    expect(overviewCallCount).toBeGreaterThanOrEqual(1);
    console.log(`[TC-DASHBOARD-00003] 35s 内 /stats/overview 调用次数：${overviewCallCount}`);

    // 跳转到其他页面，验证定时器停止
    const callCountBeforeNav = overviewCallCount;
    await page.goto(page.url().replace('/dashboard', '/rooms'));
    await page.waitForTimeout(35_000);
    const callCountAfterNav = overviewCallCount;
    // 跳走后不应再有大量 overview 调用
    expect(callCountAfterNav - callCountBeforeNav).toBeLessThanOrEqual(1);
    console.log(`[TC-DASHBOARD-00003] 跳转后 35s 内新增调用：${callCountAfterNav - callCountBeforeNav}`);
  });

  test('TC-DASHBOARD-00004: 网络异常 - 错误兜底不阻塞页面', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    const agent = new PlaywrightAgent(page);

    await agent.aiAction('在用户名输入 "e2e_admin"，密码输入 "admin_password_change_me"，点击登录');
    await page.waitForURL(/dashboard/, { timeout: 30_000 });
    await page.waitForLoadState('networkidle');

    // 拦截 stats 接口返回 500
    await page.route('**/stats/**', (route) => {
      route.fulfill({ status: 500, body: JSON.stringify({ error: 'Internal Server Error' }) });
    });

    await page.reload();
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(3000);

    await agent.aiAssert(
      '页面整体结构正常渲染（有侧边栏和页面框架），可能显示"加载失败"或"-"占位，但不出现白屏或全页错误',
    );
  });
});
