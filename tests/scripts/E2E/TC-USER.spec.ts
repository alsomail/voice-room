/**
 * 测试套件：E2E USER Web 封禁 → Android 被踢 → Web 状态刷新
 * 用例来源：doc/tests/cases/E2E/TC-USER.md
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';
import 'dotenv/config';

const URL = process.env.ADMIN_WEB_URL ?? 'http://localhost:5173';
const APP_ID = process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug';
const UID = process.env.E2E_USER_A_ID ?? '';

const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

function runMaestro(yaml: string) {
  const tmp = path.join(os.tmpdir(), `m_${Date.now()}.yaml`);
  fs.writeFileSync(tmp, yaml, 'utf-8');
  try { execSync(`maestro test "${tmp}"`, { stdio: 'inherit' }); } finally { fs.unlinkSync(tmp); }
}

test.describe('TC-USER E2E - 封禁多端闭环', () => {
  test.skip(!UID, '需要 E2E_USER_A_ID');

  test('TC-USER-00001: Web 封禁 → Android 踢下线 → Web 状态刷新', async ({ page }) => {
    // 前置：清理为正常态
    await execSync(`psql "${process.env.DATABASE_URL}" -c "UPDATE users SET banned_until=NULL WHERE id='${UID}'"`);

    // Step 1: Android 已登录，进入大厅
    runMaestro(`
appId: ${APP_ID}
---
- launchApp
- tapOn: "大厅|Hall"
- assertVisible:
    id: "room_grid"
`);

    // Step 2: Web Admin 执行封禁
    await page.goto(`${URL}/login`);
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
    await page.waitForURL(/dashboard/);
    await page.goto(`${URL}/users`);
    await agent.aiAction(`在搜索框输入 "${UID}" 并回车`);
    await agent.aiAction('点击匹配行的用户昵称，在详情抽屉中点击"封禁"按钮');
    await agent.aiAction('选择"临时"，时长选择"24 小时"，原因输入"E2E 测试"，点击"确定"');
    await agent.aiAssert('抽屉状态显示"已封禁"');

    // Step 3: DB 校验
    expect(psql(`SELECT banned_until IS NOT NULL FROM users WHERE id='${UID}'`)).toBe('t');

    // Step 4: Android 侧被踢下线并跳回登录页
    runMaestro(`
appId: ${APP_ID}
---
- launchApp
- extendedWaitUntil:
    visible: "账号被封禁|Account banned|Banned"
    timeout: 10000
- tapOn: "确定|OK"
- assertVisible:
    id: "phone_input"
`);

    // Step 5: Web 刷新列表状态同步
    await page.goto(`${URL}/users`);
    await agent.aiAction(`在搜索框输入 "${UID}" 并回车`);
    await agent.aiAssert('匹配行状态列显示红色"已封禁"标签');

    // 收尾清理
    await agent.aiAction('点击用户昵称，在抽屉中点击"解封"按钮，输入原因"清理"并二次确认');
  });
});
