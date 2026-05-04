# 全局测试约定（适用于所有 TC-*.md 用例）

> **效力**：本文件是 `doc/tests/cases/**/*.md` 全部用例的**默认前置条件与占位符约定**。除非用例显式覆写，所有 TC 文件中的「URL / Token / 用户 ID / 房间 ID」占位符均按本文映射到模块 9（E2E 测试基建）提供的运行时值。
>
> **依赖模块**：模块 9（[doc/tasks/模块9-E2E测试基建](../../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)）已交付 `npm run preflight` + `npm run db:seed` + `E2E_PROFILE` 三档环境切换；本文是用例层对模块 9 的消费契约。
>
> **闭环关系**：模块 9 自身的脚本 / 配置 / DX 测试在 [TC-INFRA-E2E.md](./API/TC-INFRA-E2E.md)。本文不重复其内容，仅作为「业务用例 → 模块 9」的桥接说明。

---

## 一、所有用例默认前置条件（隐式前置）

任何 TC-*.md 中如未显式说明环境启动方式，则隐式前置如下三条**均已通过**：

1. **环境就绪**：`E2E_PROFILE=local`（默认）已设置；`tests/scripts/env/.env.local` 已填全字段（参考 `.env.local.example` 23 个字段）。
2. **五端健康**：`npm run preflight` 退出码 0（PG / Redis / AppServer / AdminServer / Web 全 `[OK]`）。
3. **种子数据就绪**：`npm run db:seed` 已执行，`scripts/dev/.seed-output.env` 中各 ID/Token 字段非空（详见 §三 占位符映射）。

> 用例的【前置条件】章节**只需**列出**该用例额外需要**的状态（如「U1 已加入房间 R1 并占麦」「Redis 中 sms:cooldown 不存在」），不必再重复以上三条。

---

## 二、URL 占位符 → env 字段映射

所有 TC-*.md 中**禁止**直接写 `http://localhost:3000` 等硬编码 URL。统一使用占位符，由 envLoader（T-0000H）注入：

| 占位符 | 对应 env 字段 | local 默认值 | 备注 |
|--------|---------------|--------------|------|
| `${APP_SERVER_BASE_URL}` | `APP_SERVER_BASE_URL` | `http://localhost:3000` | C 端业务接口根地址 |
| `${ADMIN_SERVER_BASE_URL}` | `ADMIN_SERVER_BASE_URL` | `http://localhost:3001` | 管理端业务接口根地址 |
| `${ADMIN_WEB_URL}` | `ADMIN_WEB_URL` | `http://localhost:5173` | Web 管理后台前端 baseURL（Playwright `use.baseURL` 自动注入，WEB 用例可直接 `page.goto('/...')`） |
| `${APP_WS_URL}` | `APP_WS_URL` | `ws://localhost:3000/ws` | WS 信令地址 |
| `${ANDROID_APP_ID}` | `ANDROID_APP_ID` | `com.voiceroom.local` | Android flavor 包名（local/stg/prod 三档对应） |

**staging/prod 切换**：开发者执行 `npm run e2e:staging` 或 `npm run e2e:prod-smoke` 时，envLoader 自动从 `.env.staging` / `.env.prod` 加载，**用例文件无需任何改动**。

---

## 三、Token / ID 占位符 → seed 字段映射

所有 TC-*.md 中常见的 `TOKEN_U1` `ADMIN_TOKEN` 等占位符，统一映射到 `npm run db:seed` 产出的 `scripts/dev/.seed-output.env`（globalSetup 注入到 `process.env`）：

| 用例占位符 | seed 输出字段 | 含义 |
|------------|---------------|------|
| `TOKEN_U1` / `VALID_TOKEN` | `E2E_VALID_TOKEN` | 主测试用户 A 的 24h JWT |
| `TOKEN_U2` | `E2E_USER_B_TOKEN` | 辅助用户 B 的 JWT（连麦/送礼对象） |
| `EXPIRED_TOKEN` | `E2E_EXPIRED_TOKEN` | 90 天前签发的过期 token |
| `ADMIN_TOKEN` | `E2E_ADMIN_TOKEN` | 角色 `admin`（超级管理员） |
| `OP_TOKEN` | `E2E_OP_TOKEN` | 角色 `op`（运营） |
| `CS_TOKEN` | `E2E_CS_TOKEN` | 角色 `cs`（客服） |
| `FIN_TOKEN` | `E2E_FIN_TOKEN` | 角色 `fin`（财务） |
| `EXPIRED_ADMIN_TOKEN` | `E2E_EXPIRED_ADMIN_TOKEN` | Admin 端 401 用例使用 |
| `U1` | `E2E_USER_A_ID` | 用户 A UUID |
| `U2` | `E2E_USER_B_ID` | 用户 B UUID |
| `R1` / `ROOM_ID` | `E2E_ROOM_ID` | seed 创建的固定房间 ID |

**多用户/多房间场景**：当用例需要超出 seed 提供的 2 用户 + 1 房间时（如 U3/U4、R2/R3），用例本身的【前置条件】负责显式补充创建（e.g.「step 0：以 admin token 调 `/admin/users` 创建 U3」），并在【数据清理】负责删除。

---

## 四、Profile 切换矩阵（什么用例跑在什么 profile 下）

| 回归级别 | local | staging | prod-safe |
|----------|-------|---------|-----------|
| **P0**（核心主链路）| 全跑（`npm run e2e:local`） | 全跑（远端凭据填入后） | 仅 `@prod-safe` 标签的 read-only smoke |
| **P1**（重要功能）| 全跑 | 选跑 | 不跑 |
| **P2**（边缘兼容）| 全跑 | 不跑 | 不跑 |

**写操作类用例在 prod 自动 skip**：T-0000H 已实现 `prodSafeGuard` auto fixture，POST/PUT/DELETE 类用例在 `E2E_PROFILE=prod` 且 `E2E_ALLOW_WRITES=0` 下自动 skip 而非 fail，作者无需手工在每个用例打 skip。

**read-only smoke 用例**：必须在 `## TC-XX-NNNNN：` 标题或元数据后追加 `@prod-safe` 标签注释行（≥ 5 条用于 `npm run e2e:prod-smoke` 命中）。

---

## 五、用例执行入口（一键命令）

| 命令 | 等价行为 | 适用范围 |
|------|---------|---------|
| `npm run preflight` | 调 `scripts/dev/preflight.sh`，5 端健康检查 | 跑用例前先验证 |
| `npm run db:seed` | `E2E_PROFILE=local E2E_ALLOW_WRITES=1` 调 seed 脚本 | 首次 / 数据被破坏后重置 |
| `npm run db:reset` | 清空所有 E2E 测试数据（不影响业务表结构） | 用例间隔离 |
| `npm run e2e:local` | `E2E_PROFILE=local playwright test` | 本机全量回归 |
| `npm run e2e:staging` | `E2E_PROFILE=staging playwright test` | staging 联调 |
| `npm run e2e:prod-smoke` | `E2E_PROFILE=prod playwright test --grep "@prod-safe"` | 生产巡检（仅只读用例） |

---

## 六、新增用例的最小检查清单（Author Checklist）

在 PR 提交前，作者应确认：

- [ ] 用例文件路径符合 `doc/tests/cases/[E2E|API|AND|WEB]/TC-[模块].md` 规范。
- [ ] 用例编号在文件内从 `00001` 递增；标题、元数据、前置条件、执行步骤、数据清理五段齐全。
- [ ] **未硬编码** `localhost:3000` `localhost:3001` `localhost:5173` 任何端口；URL 用 §二 占位符。
- [ ] **未自行声明** Token 内容，统一用 §三 占位符。
- [ ] 写操作类用例对应的 fixture（如 `apiWriteRequest`）已使用，prod profile 自动 skip 路径已验证。
- [ ] 若属 read-only smoke 用例，已加 `@prod-safe` 标签。
- [ ] 用例的【数据清理】只清本用例创建的脏数据；不主动 TRUNCATE 业务表（reset 是模块 9 的职责，由 globalTeardown 完成）。
- [ ] 关键 P0 接口若涉性能断言，遵循 [TC-INFRA-E2E.md TC-00020](./API/TC-INFRA-E2E.md) 的 `100 并发 P95 ≤ 2s` 红线。

---

## 六之二、写操作真实性铁律（**铁律 6 — Wiring & Side-Effect Mandatory**）

> **背景**：2026-04-30 BUG-AUTH-WIRING — Android 登录页因 `AppNavGraph` 漏接 `LoginViewModel.Factory` 静默回退到 `NoOpAuthRepository`，按钮有"60s 倒计时"假象但**网络请求从未发出**，180 例 instrumentation 全过仍漏检。

为防止同类静默回退缺陷再次漏网，新增以下强制约束：

### 6.1 写操作类 P0 用例的副作用断言

凡 `regression_level=P0` 且涉及写操作（登录、发送验证码、创建房间、加入房间、上下麦、送礼、扣费、关注、举报）的用例，**必须**在执行步骤中包含至少一条来自下列三类的副作用断言：

| 类型 | 实现 | 用途 |
|------|------|------|
| **Server HTTP 副作用** | tail AppServer access log 5s，断言出现期望的 `METHOD path STATUS` | 证明客户端真的发出了请求且服务端处理 |
| **DB 终态** | `psql -tA -c "SELECT ..."` 断言记录已落库 | 证明业务事务真的提交 |
| **Redis/缓存终态** | `redis-cli GET/EXISTS/TTL` 断言键存在且符合预期 | 证明分布式态切换 |

仅含 `assertVisible` UI 文案断言的写操作类 P0 用例，PR 自动 reject。

### 6.2 装配契约（Wiring Contract）层强制覆盖

以下场景**必须**有对应用例落在 [E2E/TC-WIRING.md](./E2E/TC-WIRING.md)，并以**真实 `MainActivity` 启动**（禁止 `composeTestRule.setContent` 隔离渲染）：

- 任何在 `AppNavGraph` 注册的路由对应 ViewModel 的关键依赖（Repository / RtcPort / AnalyticsPort / TokenManager / WsClient）注入正确性。
- 任何使用"NoOp / Preview Stub / Fake"作为兜底实现的接口（避免 DI 漏接静默回退）。
- 跨 Screen 的核心交互链（登录 → 大厅 → 房间 → 麦位 → 送礼）首尾相接的最小可验证闭环。

### 6.3 反模式黑名单（PR Review 必查）

| 反模式 | 替代方案 |
|--------|---------|
| Maestro/E2E 脚本硬编码 `appId: com.voiceroom.debug` 等具体包名 | 使用 `${ANDROID_APP_ID}` 占位符，由 envLoader 注入 |
| `tapOn: index: 0` 等基于节点序号的定位 | 使用 testTag (`id:`) 或稳定可见文本 |
| 仅 `assertVisible: "VoiceRoom\|语聊房"` 判定登录成功 | 必须配合 access-log + DB 双断言 |
| `composeTestRule.setContent { Screen(viewModel = fake) }` 用于 P0 主流程 | 该写法仅允许用于纯 UI 视觉/文案回归（P1/P2），主流程必须走真实 `MainActivity` |
| `NoOpXxxRepository` 在 release/local flavor 仍可被链接到 | 应隔离到 `previewDebug` source set，避免运行时静默回退 |

---

## 六之三、E2E 框架统一铁律（**铁律 7 — Midscene-Only E2E**）

> **背景**：本仓库 E2E 历史上同时存在 Maestro yaml（Android）+ Playwright spec（Web）+ 自研脚本三种形态，导致脚本碎片化、断言能力参差、AI 视觉推理（Midscene）只覆盖 Web。2026-04-30 决议：**所有 E2E 用例的视觉/交互/断言层一律收敛到 Midscene**，跨进程的副作用断言（DB / Redis / log）由 Playwright `test.step` 内调用 shell 完成。

### 7.1 框架对应矩阵（强制）

| 测试层 | 端 | **唯一允许的框架** | SDK | 用途 |
|--------|----|--------------------|-----|------|
| E2E 视觉/交互 | Web 浏览器 | **Playwright + `@midscene/web`** | `agentForPage()` | 所有 Web 端 E2E |
| E2E 视觉/交互 | Android 真机/模拟器 | **Playwright + `@midscene/android`** | `agentFromAdbDevice()` | 所有 Android 端 E2E |
| 跨端副作用断言 | DB / Redis / AppServer log | **Playwright `test.step` + `child_process.execSync`** | `psql` / `redis-cli` / `tail` | 配合上面两层做铁律 6 副作用断言 |
| Compose 单元/视觉 | Android（仅 P1/P2 视觉/文案） | Compose Test (`composeTestRule.setContent`) | — | **不计入 E2E**，仅作组件级回归 |

### 7.2 禁用清单（**红线**）

| 框架 / 写法 | 状态 | 替代方案 |
|------------|------|---------|
| **Maestro yaml**（`tests/scripts/AND/*.yaml`） | ❌ **废弃**，新增用例禁止使用 | 改写为 `tests/scripts/AND/*.spec.ts`（Midscene Android Agent） |
| Espresso / UIAutomator 直接调用 | ❌ 禁用于 E2E 层 | 同上 |
| 在 E2E 用例中 `composeTestRule.setContent` | ❌ 禁用 | 真实 `MainActivity` + Midscene Android Agent |
| 自研 adb shell input 拼接脚本 | ❌ 禁用 | Midscene Android Agent 的 `aiTap/aiInput/aiAssert` |
| `tapOn: index: N` / `tapOn: text` 等 Maestro 原语 | ❌ 禁用 | `agent.aiTap('金色发光的"获取验证码"按钮')` 自然语言定位 |

### 7.3 Midscene 用例的最小骨架

**Web 端**（`tests/scripts/WEB/TC-XXX.spec.ts`）：
```ts
import { test, expect } from '@playwright/test';
import { PlaywrightAiFixture } from '@midscene/web/playwright';

test.use(PlaywrightAiFixture());

test('TC-XXX-00001: ...', async ({ page, ai, aiAssert, aiTap, aiInput }) => {
  await page.goto('/login');
  await aiInput('+966500000900', '手机号输入框');
  await aiTap('"获取验证码" 按钮');
  await aiAssert('按钮文案变为倒计时 "60s 后重发"');
  // 跨端副作用断言
  await test.step('AppServer log 副作用', () => {
    const log = execSync('tail -n 50 /tmp/server.log').toString();
    expect(log).toMatch(/POST \/api\/v1\/auth\/verification-codes.*200/);
  });
});
```

**Android 端**（`tests/scripts/AND/TC-XXX.spec.ts`）：
```ts
import { test, expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';

test('TC-AUTH-00003: 新用户登录闭环', async () => {
  const agent = await agentFromAdbDevice(process.env.ADB_DEVICE_ID, {
    androidAdbPath: 'adb',
  });
  await agent.launchApp(process.env.ANDROID_APP_ID!);
  await agent.aiInput('+966500000900', '手机号输入框');
  await agent.aiTap('"获取验证码" 按钮');
  await agent.aiAssert('按钮变为 "60s 后重发"');
  // 副作用断言走 Playwright test.step + execSync（同上）
});
```

### 7.4 依赖与 npm script 约定

| 依赖 | 用途 | 安装位置 |
|------|------|---------|
| `@midscene/web` | Web E2E（已装 ^1.7.5） | `package.json devDependencies` |
| `@midscene/android` | Android E2E | **待新增**（`npm i -D @midscene/android`） |
| `appium`（可选） | Midscene Android 底层之一 | 不强制；优先走 `agentFromAdbDevice` 纯 ADB 路径 |

新增 npm scripts 入口（待落盘）：
```jsonc
{
  "e2e:android": "cross-env E2E_PROFILE=local playwright test tests/scripts/AND tests/scripts/E2E"
}
```

### 7.5 迁移方针（对存量 `tests/scripts/AND/*.yaml`）

1. **冻结新增**：自本铁律落盘起，不再新增任何 `.yaml` Maestro 用例；任何 PR 引入 yaml 自动 reject。
2. **逐步重写**：现存 10 个 `tests/scripts/AND/TC-*.yaml` 逐 Task 改写为 `TC-*.spec.ts`（Midscene Android Agent），按 P0 → P1 → P2 优先级执行。
3. **过渡期标识**：未迁移的 yaml 顶部必须有 `# DEPRECATED: 待按铁律 7 迁移到 Midscene` 注释；CI 不再调度 yaml。
4. **测试报告**：所有 E2E 报告中"用例总数"仅统计 Midscene spec，yaml 计为"废弃存量"独立列出。

### 7.6 Author Checklist 增补

新增 / 修改 E2E 用例的 PR 必须再勾选：

- [ ] 用例位于 `tests/scripts/{WEB,AND,E2E}/TC-*.spec.ts`，**未**新增 `.yaml`。
- [ ] 视觉与交互**全部**通过 Midscene 的 `aiTap / aiInput / aiAssert / aiQuery` 完成，未直调 `page.click(selector)` / `adb shell input` 等底层原语。
- [ ] 涉及写操作的 P0 用例满足铁律 6（含 access-log / DB / Redis 副作用断言）。
- [ ] 已用 envLoader 占位符（`${ANDROID_APP_ID}` / `${APP_SERVER_BASE_URL}` 等），未硬编码 appId / URL。
- [ ] Midscene Key 缺失时用例自动 skip 而非 fail（沿用 `midsceneReady` fixture）。

---

## 七、与 TC-INFRA-E2E.md 的边界

| 测试目标 | 归属文件 |
|----------|---------|
| 模块 9 自身脚本 / config / env 模板的正确性 | [TC-INFRA-E2E.md](./API/TC-INFRA-E2E.md)（20 条用例） |
| 模块 0 工程基建（Docker / shared crate / DB 权限 / CI） | [TC-INFRA.md](./API/TC-INFRA.md) |
| 业务功能用例（消费模块 9 提供的能力） | 其他全部 TC-*.md |

**铁律**：业务用例**禁止**重复测试模块 9 已覆盖的内容（如 preflight 退出码、seed 幂等性、envLoader fail-fast）；这些是模块 9 的内部不变量，业务用例只消费、不验证。
