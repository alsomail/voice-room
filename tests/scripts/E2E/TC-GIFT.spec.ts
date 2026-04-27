/**
 * 测试套件：E2E GIFT 跨端打赏闭环
 * 用例来源：doc/tests/cases/E2E/TC-GIFT.md
 * 场景：Android U1 → 麦位 U2 送礼，WS 推送 + DB 事务 + Web Dashboard 统计
 */
import { test, expect } from '@playwright/test';
import { PlaywrightAgent } from '@midscene/web/playwright';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';

const APP = process.env.APP_SERVER_BASE_URL!;
const APP_ID = process.env.ANDROID_APP_ID ?? 'com.voiceroom.debug';
const A = process.env.E2E_USER_A_ID ?? '';
const B = process.env.E2E_USER_B_ID ?? '';
const ROOM = process.env.E2E_ROOM_ID ?? '';

const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

function runMaestro(yaml: string) {
  const tmp = path.join(os.tmpdir(), `maestro_gift_${Date.now()}.yaml`);
  fs.writeFileSync(tmp, yaml, 'utf-8');
  try { execSync(`maestro test "${tmp}"`, { stdio: 'inherit' }); } finally { fs.unlinkSync(tmp); }
}

test.describe('TC-GIFT E2E - 跨端打赏', () => {
  test.skip(!A || !B || !ROOM, '需要 UID/ROOM');

  test('TC-GIFT-00001: Android U1 向麦位 U2 送礼 多端闭环', async ({ page }) => {
    // Step 0: 记录初值
    const a0 = Number(psql(`SELECT coin_balance FROM users WHERE id='${A}'`));
    const b0 = Number(psql(`SELECT coin_balance FROM users WHERE id='${B}'`));

    // Step 1-3: Android 打开礼物面板并送出一个火箭（价格 500）
    runMaestro(`
appId: ${APP_ID}
---
- launchApp
- tapOn: "大厅|Hall"
- tapOn:
    index: 0
- tapOn:
    id: "gift_button"
- tapOn: "火箭|Rocket"
- tapOn:
    id: "send_gift_button"
- extendedWaitUntil:
    visible:
      id: "gift_effect_l3"
    timeout: 5000
`);

    // Step 4: DB 事务校验
    const a1 = Number(psql(`SELECT coin_balance FROM users WHERE id='${A}'`));
    const b1 = Number(psql(`SELECT coin_balance FROM users WHERE id='${B}'`));
    expect(a0 - a1).toBe(500);
    expect(b1 - b0).toBeGreaterThan(0);
    const tx = Number(psql(
      `SELECT count(*) FROM transactions WHERE user_id='${A}' AND delta=-500 ORDER BY created_at DESC LIMIT 1`,
    ));
    expect(tx).toBe(1);

    // Step 5: Web Dashboard 增量
    await page.goto('/login');
    const agent = new PlaywrightAgent(page);
    await agent.aiAction('在用户名输入 "admin_op"，密码输入 "Pass@123"，点击登录');
    await page.waitForURL(/dashboard/);
    await page.reload();
    await agent.aiAssert('"今日打赏总额"卡片的数字较之前增加，且不小于 500');
  });
});
