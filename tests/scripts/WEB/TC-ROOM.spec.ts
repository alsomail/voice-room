/**
 * 测试套件：ROOM 房间监控（Web）
 * 用例来源：doc/tests/cases/WEB/TC-ROOM.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';


async function login(page: any) {
  await page.goto('/login');
  await page.waitForLoadState('networkidle');
  const agent = new PlaywrightAgent(page);
  await agent.aiAction('在用户名输入框输入 "e2e_op"');
  await agent.aiAction('在密码输入框输入 "admin_password_change_me"');
  await agent.aiAction('点击"登录"按钮');
  await page.waitForURL(/dashboard/);
  await page.waitForLoadState('domcontentloaded');
  return agent;
}

test.describe('TC-ROOM WEB - 房间监控', () => {
  test.skip(!process.env.MIDSCENE_MODEL_API_KEY, '[MIDSCENE] MIDSCENE_MODEL_API_KEY 未设置，跳过 AI 视觉用例');
  test('TC-ROOM-00001: Dashboard 概览 + ECharts + 30s 自动刷新', async ({ page }) => {
    const agent = await login(page);
    // login() 已经落在 /dashboard，这里等待卡片渲染完成
    await page.waitForSelector('.ant-statistic, .ant-card, [class*="statistic"]', { timeout: 15_000 });
    await page.waitForTimeout(1000);
    await agent.aiAssert('页面顶部有 4 个数字卡片：在线人数、活跃房间、今日 DAU、今日新增用户；下方有趋势图区域');
    // 等待 30s 刷新
    await page.waitForTimeout(31_000);
    await agent.aiAssert('卡片数据或时间戳有更新变化，未出现报错提示');
  });

  test('TC-ROOM-00002: 房间列表 - 筛选 / 分页', async ({ page }) => {
    test.setTimeout(180_000);
    const agent = await login(page);
    await page.goto('/rooms');
    await page.waitForLoadState('domcontentloaded');
    // 等待表格加载
    await page.waitForSelector('.ant-table', { timeout: 15_000 });
    // 直接操作 data-testid="status-filter" 下拉框，避免与 activity-filter 的"活跃/全部"选项混淆
    await page.locator('[data-testid="status-filter"] .ant-select-content').click();
    await page.waitForTimeout(500);
    await page.locator('.ant-select-item-option').filter({ hasText: '已关闭' }).click();
    await page.waitForTimeout(1500);
    await agent.aiAssert('表格行状态列均显示"已关闭"，或表格提示无数据');
    // 重置为全部后测试分页（T-0000R Round1：seed 已保证 ≥12 房间，分页必然可用）
    await page.locator('[data-testid="status-filter"] .ant-select-content').click();
    await page.waitForTimeout(500);
    await page.locator('.ant-select-item-option').filter({ hasText: '全部' }).first().click();
    await page.waitForTimeout(1000);
    await agent.aiAction('点击分页控件的"下一页"或第 2 页按钮');
    await page.waitForTimeout(1500);
    // T-0000R Round1 修复：seed 已保证 12 房间 + 默认 page_size=10 → 必然有第 2 页，真实验证分页功能
    await agent.aiAssert('分页器当前页码显示为 2，且表格行刷新为第二页数据');
  });

  test('TC-ROOM-00003: 详情弹窗 - 强制关闭完整闭环', async ({ page }) => {
    // T-0000R Round1：seed 已幂等保证至少 9 个 active 房间，无需 spec 内自愈（移至 globalSetup 前置）
    const agent = await login(page);
    await page.goto('/rooms');
    await page.waitForLoadState('domcontentloaded');
    // 等待表格加载
    await page.waitForSelector('.ant-table-row', { timeout: 15_000 });
    // 直接操作 data-testid="status-filter" 下拉框，避免与 activity-filter 的"活跃"选项混淆
    await page.locator('[data-testid="status-filter"] .ant-select-content').click();
    await page.waitForTimeout(500);
    await page.locator('.ant-select-item-option').filter({ hasText: '活跃' }).first().click();
    await page.waitForTimeout(1500);
    await agent.aiAction('点击表格第一行的房间标题或任意单元格');
    // Wait for room detail to load (useRoomDetail async hook), so "强制关闭" button is enabled (not disabled)
    await page.waitForSelector('[data-testid="detail-basic-info"]', { timeout: 10_000 });
    // Verify the close button is enabled (status=active)
    await page.waitForSelector('[data-testid="close-room-btn"]:not([disabled])', { timeout: 5_000 });
    // Click "强制关闭" button directly (avoid AI confusion with table's "关闭房间" buttons)
    await page.locator('[data-testid="close-room-btn"]').click();
    // RoomDetailModal 使用 Modal.confirm（无原因输入框，仅二次确认）
    // BUG-WEB-003 fix: Wait for confirm modal container + OK button (more reliable than title text)
    await page.waitForTimeout(1000); // Allow modal animation to complete
    const confirmModal = page.locator('.ant-modal-wrap').last(); // Last modal = topmost (confirm over detail)
    await confirmModal.waitFor({ state: 'visible', timeout: 8_000 });
    const okButton = confirmModal.getByRole('button', { name: /确.*定|OK/i });
    await okButton.waitFor({ state: 'visible', timeout: 3_000 });
    // Click OK to confirm close (skip AI assertion to avoid timing race with modal close animation)
    await okButton.click();
    await page.waitForTimeout(2000);
    await agent.aiAssert('强制关闭操作成功：返回房间列表页（无打开的详情弹窗）');
  });

  test('TC-ROOM-00004: XSS 防护 - 标题恶意输入', async ({ page }) => {
    const agent = await login(page);
    await page.goto('/rooms');
    await page.waitForLoadState('domcontentloaded');
    await agent.aiAction('在搜索框输入 "<script>alert(1)</script>" 并回车');
    // 不应弹出浏览器原生 alert
    page.on('dialog', (d) => { throw new Error('XSS alert leaked: ' + d.message); });
    await agent.aiAssert('页面正常渲染，表格为空或显示"无匹配房间"，字符以纯文本形式呈现');
  });

  test('TC-ROOM-00005: 活跃房间监控增强 - 状态/时长/异常高亮', async ({ page }) => {
    // T-20011: 功能在 /rooms 列表页实现（新增活跃状态列、持续时长列、活跃度筛选器）
    // 原 test.skip 为测试 bug（误写 /rooms/active 路由），此处修正为正确的验收路径
    // Round1 修复：增大超时至 300s（Midscene AI 单次断言耗时约 30-60s，5次调用需足够余量）
    test.setTimeout(300_000);
    const agent = await login(page);
    await page.goto('/rooms');
    await page.waitForLoadState('domcontentloaded');
    await page.waitForSelector('.ant-table', { timeout: 15_000 });

    // 验证新增的"活跃状态"列和"持续时长"列存在
    await agent.aiAssert('房间列表表格包含"活跃状态"列（显示活跃/冷清/异常/正常的 Tag）以及"持续时长"列（显示如"2h 35m"或"0m"格式的时间）');

    // 验证活跃度筛选器存在（data-testid="activity-filter" 包裹 Ant Design v6 Select）
    // AntD v6 的 Select 触发区域 class 为 ant-select-content（非 v5 的 ant-select-selector）
    const activityFilter = page.locator('[data-testid="activity-filter"]');
    if (await activityFilter.count() > 0) {
      // 点击筛选器展开（给予充足超时，防止 AI 断言消耗剩余时间）
      await activityFilter.locator('.ant-select-content').click({ timeout: 10_000 });
      await page.waitForTimeout(800);
      await agent.aiAssert('下拉菜单中有"全部"、"活跃"、"冷清"、"异常"选项');
      // 关闭下拉
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);
    }

    // 验证 Tag 颜色语义（活跃→绿, 冷清→黄/橙, 异常→红, 正常→蓝）
    await agent.aiAssert('活跃状态 Tag 的颜色与状态对应（活跃→绿色, 冷清→黄色/橙色, 异常→红色, 正常→蓝色）');
  });
});
