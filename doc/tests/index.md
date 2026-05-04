# E2E 测试文档索引

> **负责人**：QA / Infrastructure Team  
> **最后更新**：2026-06-03

---

## 文档清单

| 文档 | 内容 | 关联 Task |
|------|------|----------|
| [MIDSCENE_SETUP.md](./MIDSCENE_SETUP.md) | Midscene LLM 三形态配置指南（OpenAI 直连/Azure/中转）+ GitHub Actions Secret 注入 + 缺 Key 自动 skip + runtime json 脱敏安全规约 | T-0000K |
| [E2E_RUNBOOK.md](./E2E_RUNBOOK.md) | E2E 启动 SOP：冷启动 5 步 + 一键命令矩阵 + preflight 5 端排查表 + staging/prod-safe 凭据流程 + CI Secrets + FAQ | T-0000L |

---

## 快速开始

### 本地 WEB 用例执行（5 分钟）

1. **填入 Midscene Key**：参考 [MIDSCENE_SETUP.md §1](./MIDSCENE_SETUP.md#一-三形态字段冻结表) 选择部署形态，将 API Key 填入 `tests/scripts/env/.env.local`
2. **一键启动**：
   ```bash
   npm run preflight    # 5 端健康检查
   npm run e2e:local    # 运行本地 E2E 用例
   ```
3. **缺 Key 时**：WEB 用例自动 skip，无需修改代码

### CI 环境接入

参考 [MIDSCENE_SETUP.md §3](./MIDSCENE_SETUP.md#三-github-actions-secret-注入与-ci-流程) GitHub Actions Secret 注入示例

---

## 测试用例分类

- **API 用例** (`tests/scripts/API/`): 后端接口功能测试（Playwright `request` fixture）
- **WEB 用例** (`tests/scripts/WEB/`): Web 端 E2E（Playwright + `@midscene/web`）
- **AND 用例** (`tests/scripts/AND/`): Android 端 E2E（Playwright + `@midscene/android`，`agentFromAdbDevice`）
- **E2E 跨端用例** (`tests/scripts/E2E/`): 跨端联调（Web/Android + DB/Redis/AppServer log 副作用断言）
- **Admin WEB 用例** (`tests/scripts/ADMIN_WEB/`): 管理后台 E2E（Playwright + `@midscene/web`）

> ⚠️ **铁律 7（2026-04-30）**：所有 E2E 用例的视觉与交互层**必须**经由 Midscene 完成（`aiTap / aiInput / aiAssert / aiQuery`）。Maestro yaml（`tests/scripts/AND/*.yaml`）已废弃，仅保留为历史参考，CI 不再调度。详见 [cases/_README.md §六之三](./cases/_README.md)。

---

## Midscene 用例骨架模板（铁律 7）

> 所有新增/重写的 E2E 用例**必须**遵循以下两套骨架之一。完整可运行样板见 [tests/scripts/AND/TC-AUTH.spec.ts](../../tests/scripts/AND/TC-AUTH.spec.ts)。

### A. Android E2E 模板（`tests/scripts/AND/TC-*.spec.ts`）

```ts
/**
 * 测试套件：<模块> （Android）
 * 用例来源：doc/tests/cases/AND/TC-<模块>.md
 */
import { execSync } from 'child_process';
import { expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';
import { test } from '../support/fixtures'; // 复用 prodSafeGuard / midsceneReady auto fixture

// psql / redis 工厂：连接参数由 e2eEnv 传入，不依赖 process.env
// （Playwright worker 是独立进程，env 通过 .e2e-runtime.json → e2eEnv fixture 传递，而非 process.env 继承）
const psql  = (dbUrl: string, sql: string) =>
  execSync(`psql "${dbUrl}" -tA -c "${sql.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();
const redis = (redisUrl: string, cmd: string) =>
  execSync(`redis-cli -u "${redisUrl}" ${cmd}`, { encoding: 'utf-8' }).trim();

test.describe('TC-<MODULE>', () => {
  test.setTimeout(180_000); // Midscene 视觉模型推理较慢

  test('TC-<MODULE>-00001: <场景描述>', async ({ e2eEnv }) => {
    // ✅ 从 fixture 读取连接参数（工作目录无关，始终从根目录 .e2e-runtime.json 加载）
    const ANDROID_APP_ID = e2eEnv.androidAppId;
    if (!ANDROID_APP_ID) throw new Error('e2eEnv.androidAppId 未配置 — 请在 tests/scripts/env/.env.local 中设置 ANDROID_APP_ID');
    const DATABASE_URL   = e2eEnv.databaseUrl!;
    const REDIS_URL      = e2eEnv.redisUrl ?? 'redis://localhost:6379';
    // ADB_DEVICE_ID 不在 E2EEnv，允许从 process.env 读（非敏感配置，cross-env / shell export 均可注入）
    const ADB_DEVICE_ID  = process.env.ADB_DEVICE_ID || undefined;
    const adbPrefix      = ADB_DEVICE_ID ? `adb -s ${ADB_DEVICE_ID}` : 'adb';

    // 前置清理
    await test.step('precondition', () => { psql(DATABASE_URL, `DELETE FROM ...`); });

    const agent = await agentFromAdbDevice(ADB_DEVICE_ID, { androidAdbPath: 'adb' });

    try {
      await test.step('launch', async () => {
        execSync(`${adbPrefix} shell pm clear ${ANDROID_APP_ID}`);
        await agent.launchApp(ANDROID_APP_ID);
      });

      // 视觉/交互层 — 全部走 Midscene，禁止 adb shell input
      await agent.aiInput('500000100', '手机号输入框');
      await agent.aiTap('"获取验证码" 按钮');
      await agent.aiAssert('按钮文案变为 "60s 后重发"');

      // 副作用断言（铁律 6）— Redis / DB 任选一条以上
      await test.step('side-effect: Redis', () => {
        expect(Number(redis(REDIS_URL, `TTL sms:code:+966500000100`))).toBeGreaterThan(0);
      });
    } finally {
      // 数据清理（try/finally 保证清理必执行）
      await test.step('cleanup', () => { psql(DATABASE_URL, `DELETE FROM ...`); });
    }
  });
});
```

### B. Web E2E 模板（`tests/scripts/WEB/TC-*.spec.ts`）

```ts
import { expect } from '@playwright/test';
import { PlaywrightAiFixture } from '@midscene/web/playwright';
import { test as base } from '../support/fixtures';

const test = base.extend(PlaywrightAiFixture());

test('TC-XXX-00001: <场景>', async ({ page, ai, aiAssert, aiTap, aiInput }) => {
  await page.goto('/login');
  await aiInput('+966500000900', '手机号输入框');
  await aiTap('"获取验证码" 按钮');
  await aiAssert('按钮文案变为倒计时');
  // 跨端副作用断言：可在此 step 内 execSync('psql -c ...')
});
```

### 模板使用要点（强制）

| 项 | 要求 |
|----|------|
| 视觉/交互 | **必须**走 `agent.aiTap / aiInput / aiAssert / aiQuery`，禁止 `page.click(selector)` 或 `adb shell input` |
| 元素定位描述 | 自然语言描述（如 `'金色发光的"获取验证码"按钮'`），禁止 CSS 选择器 / 节点 index |
| 副作用断言 | P0 写操作类用例必须含 ≥1 条 `psql` / `redis-cli` / AppServer log 断言（铁律 6） |
| 占位符 | URL 与 appId 走 envLoader（`ANDROID_APP_ID` / `APP_SERVER_BASE_URL` / `DATABASE_URL`），禁止硬编码 |
| 缺 Key | 自动 skip：`midsceneReady` auto fixture 在 `MIDSCENE_MODEL_API_KEY` 为空时跳过 WEB/AND/E2E 子树 |
| 超时 | Android E2E 必须 `test.setTimeout(180_000)` 起步，视觉推理慢 |
| 清理 | 每例必须 `try/finally` 包裹，`finally` 内 `test.step('cleanup', ...)` 清脏数据 |

---

## Android 端调试经验（TC-AUTH-00003 落地总结，2026-04-30）

### 1. Playwright 项目隔离

`playwright.config.ts` 中 `android` project 必须与浏览器 projects 互相排除，否则 AND/ 目录被三个浏览器各跑一次：

```ts
projects: [
  { name: 'chromium', testIgnore: ['**/AND/**'], use: { ...devices['Desktop Chrome'] } },
  { name: 'firefox',  testIgnore: ['**/AND/**'], use: { ...devices['Desktop Firefox'] } },
  { name: 'webkit',   testIgnore: ['**/AND/**'], use: { ...devices['Desktop Safari'] } },
  { name: 'android',  testMatch: ['**/AND/**'] },  // 无 use.browser
]
```

执行时必须加 `--project=android`，否则被浏览器 project 捡走报错：
```bash
npm run e2e:android -- tests/scripts/AND/TC-AUTH.spec.ts --project=android
```

### 2. 冷启动必须处理首次同意弹窗

`pm clear` 清空数据后，App 首次启动会弹"数据收集通知"弹窗，阻塞登录页加载。`launch` 后立即等弹窗并关闭：

```ts
await agent.launch(ANDROID_APP_ID);
await agent.aiWaitFor('界面上有可交互的按钮或输入框（弹窗或登录页均可）', { timeoutMs: 15_000 });
const hasConsentDialog = await agent.aiBoolean('当前界面是否存在数据收集通知、隐私政策或权限请求弹窗？');
if (hasConsentDialog) {
  await agent.aiTap('"同意" 或 "确定" 或 "接受" 按钮（关闭弹窗）');
}
await agent.aiWaitFor('手机号输入框可见，登录页面已加载完成', { timeoutMs: 10_000 });
```

### 3. 数据库清理：FK 顺序（先删子表再删主表）

`rooms.owner_id → users.id` 有外键约束。清理必须先删 rooms：

```ts
psql(DATABASE_URL, `DELETE FROM rooms WHERE owner_id = (SELECT id FROM users WHERE phone='${phone}' LIMIT 1)`);
psql(DATABASE_URL, `DELETE FROM users WHERE phone='${phone}'`);
```

### 4. Redis：用 `redisExecSync`，SMS 验证码是 Hash 结构

本地 macOS 没有 `redis-cli`，Redis 跑在 Docker 容器 `vr-redis`。禁止直接调 `redis-cli`，使用项目工具函数：

```ts
import { redisExecSync } from '../support/redisCli'; // 自动路由到 docker exec vr-redis redis-cli
```

AppServer 用 `HSET` 存 SMS 验证码（Hash，含 `code` + `attempts` 字段），**不能用 `GET`**：

```ts
const code = redisExecSync(['HGET', `sms:code:${phone}`, 'code']);     // ✅ 读
redisExecSync(['HSET', `sms:code:${phone}`, 'code', '123456']);        // ✅ 覆写
redisExecSync(['DEL', `sms:code:${phone}`, `sms:cooldown:${phone}`]); // ✅ 清理
```

用 `GET` 读 Hash key 会报 `WRONGTYPE` 错误。

### 5. seed 子进程缺 JWT_SECRET（exit=22）

`loadE2EEnv` 是纯函数，不写 `process.env`。seed 以子进程运行，默认拿不到 profile 文件里的 JWT 密钥。`globalSetup` 调用 seed 前需显式注入：

```ts
const rawProfileVars = dotenv.parse(fs.readFileSync(`.env.${profile}`));
const jwtExtra = Object.fromEntries(
  ['JWT_SECRET','APP_JWT_SECRET','ADMIN_JWT_SECRET']
    .filter(k => process.env[k] ?? rawProfileVars[k])
    .map(k => [k, process.env[k] ?? rawProfileVars[k]])
);
// runShell(seed-e2e.sh, { env: { ...process.env, ...jwtExtra } })
```

密钥在 `tests/scripts/env/.env.local` 中配置，与 `config/local.toml` 保持一致。

### 6. 区分"测试基建 bug"与"真实应用 bug"

| 现象 | 类型 | 处置 |
|------|------|------|
| `redis-cli: command not found`、FK 错误、`JWT_SECRET is required` | 测试脚本/基建 bug | 修脚本或 globalSetup |
| `aiWaitFor timeout`、弹窗遮挡登录页 | 时序/等待问题 | 加等待/弹窗处理 |
| 操作链路全通、但功能结果与预期不符（如 force-stop 后显示登录页） | **真实应用 bug** | 记录 bug 上报 TDD，**脚本断言不动** |

---

## 用例编写约定

> **新增/修改任何 TC-*.md 用例前，先读 [cases/_README.md](./cases/_README.md)**：声明全局隐式前置（preflight + seed + profile）、URL/Token 占位符 → env 字段映射、profile 切换矩阵、作者检查清单、铁律 6（写操作真实性）与铁律 7（Midscene-Only E2E）。该约定让现有用例无需逐个重写就能受益于模块 9 的多环境基建。

## 模块 9（E2E 测试基建）专项用例

> 模块 9 自身的测试基建（env 模板 / Seed/Reset/Preflight / globalSetup / 多端 config / npm scripts / Midscene / RUNBOOK）作为 CLI/脚本级集成测试，统一收口于 [cases/API/TC-INFRA-E2E.md](./cases/API/TC-INFRA-E2E.md)，共 20 条用例覆盖 T-0000E~L + T-00040 + T-10020 + T-20020 + T-30050。
>
> 模块 0 工程基建（Docker Compose / shared crate / DB 权限 / CI）见 [cases/API/TC-INFRA.md](./cases/API/TC-INFRA.md)。

---

## 关键约定

| 约定 | 说明 |
|------|------|
| **Key 缺失行为** | 对 `tests/scripts/{WEB,AND,E2E}/` 三类生效：自动 skip 而非 fail；skip reason = `'[MIDSCENE] api key missing — skipped'`（铁律 7 后扩展） |
| **Security** | API Key 永不入 `.e2e-runtime.json`；CI 日志脱敏；错误信息 Key 脱敏 |
| **多环境支持** | local/staging/prod 三档环境，缺 Key 时 Midscene 依赖套件自动 skip（不影响 API 等纯接口套件） |
| **写操作真实性** | P0 写操作必须含 ≥1 条 DB / Redis / AppServer log 副作用断言（铁律 6） |
| **E2E 框架** | 视觉与交互层一律走 Midscene；不允许新写 Maestro yaml 或 `composeTestRule.setContent`（铁律 7） |
