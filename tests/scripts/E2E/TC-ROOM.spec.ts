/**
 * 测试套件：E2E ROOM 强制关闭闭环
 * 用例来源：doc/tests/cases/E2E/TC-ROOM.md
 * 场景：Web 管理员强制关房 → Android 端收到 RoomClosed → 回大厅
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';

const APP = process.env.APP_SERVER_BASE_URL!;
const APP_ID = process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug';
const T = process.env.E2E_VALID_TOKEN ?? '';

const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

function runMaestro(yaml: string) {
  const tmp = path.join(os.tmpdir(), `m_${Date.now()}.yaml`);
  fs.writeFileSync(tmp, yaml, 'utf-8');
  try { execSync(`maestro test "${tmp}"`, { stdio: 'inherit' }); } finally { fs.unlinkSync(tmp); }
}

test.describe('TC-ROOM E2E - Web 强制关闭 → App 被动退出', () => {
  test('TC-ROOM-00001: 强制关房闭环', async ({ request, page }) => {
    test.skip(!T, '需要 E2E_VALID_TOKEN');

    // Step 1: 通过 AppServer 用 C 端 token 创建一个房间
    const created = await request.post(`${APP}/api/v1/rooms`, {
      headers: { Authorization: `Bearer ${T}` },
      data: { title: `e2e_fc_${Date.now()}`, cover: 1, type: 'chat' },
    });
    expect(created.status()).toBe(201);
    const rid = (await created.json()).data.id;

    // Step 2: Android 进入该房间
    runMaestro(`
appId: ${APP_ID}
---
- launchApp
- tapOn: "大厅|Hall"
- tapOn:
    text: "e2e_fc_"
- extendedWaitUntil:
    visible:
      id: "room_screen_root"
    timeout: 5000
`);

    // Step 3: Web 管理员强制关房
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
    await page.waitForURL(/dashboard/);
    await page.goto('/rooms');
    await agent.aiAction(`在搜索框输入房间 ID "${rid}" 并回车`);
    await agent.aiAction('点击结果行的房间标题');
    await agent.aiAction('在右侧抽屉点击"强制关闭房间"按钮');
    await agent.aiAction('在确认弹窗原因输入"E2E 测试"，点击"确定"');
    await agent.aiAssert('抽屉状态变为"已关闭"');

    // Step 4: 断言 DB
    expect(psql(`SELECT status FROM rooms WHERE id='${rid}'`)).toBe('closed');

    // Step 5: Android 被动退出 + 回大厅
    runMaestro(`
appId: ${APP_ID}
---
- launchApp
- extendedWaitUntil:
    visible: "房间已关闭|Room Closed"
    timeout: 10000
- tapOn: "确定|OK"
- assertVisible:
    id: "room_grid"
`);
  });
});
