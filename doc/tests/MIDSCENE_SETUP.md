# Midscene LLM 配置接入指南

> 关联：[T-0000K TDS](../tds/infra/T-0000K.md) / [T-0000F 三档 env](../tds/infra/T-0000F.md) / [T-0000H envLoader](../tds/infra/T-0000H.md) / 用例约定铁律 7（[_README.md §六之三](./cases/_README.md)）
> 适用范围：**所有 E2E 用例**（铁律 7：E2E 框架统一为 Midscene）
> - `tests/scripts/WEB/TC-*.spec.ts` —— `@midscene/web/playwright`
> - `tests/scripts/AND/TC-*.spec.ts` —— `@midscene/android` + `agentFromAdbDevice`
> - `tests/scripts/E2E/TC-*.spec.ts` —— 跨端联调（Web + Android + DB / Redis / log 副作用断言）
>
> 一句话目标：**5 分钟内让 Web/Android E2E 真跑通；缺 Key 时自动 skip 而非红条**。

---

## 1. 三种部署形态字段映射（冻结表）

Midscene 通过 `process.env` 读取配置，本仓库通过 `tests/scripts/env/.env.<profile>` + `envLoader` 注入。下表为三形态必填 / 可选字段冻结清单。

### 1.1 形态 A：OpenAI 直连（默认）

| 字段 | 必填 | 示例 | 说明 |
|---|---|---|---|
| `MIDSCENE_MODEL_API_KEY` | ✅ | `sk-***`（永远走 Secret） | OpenAI API Key |
| `MIDSCENE_MODEL_NAME` | ✅ | `gpt-4o` | 视觉模型名 |
| `MIDSCENE_OPENAI_BASE_URL` | ⬜ | （留空） | 留空走默认 `https://api.openai.com/v1` |
| `MIDSCENE_CACHE` | ⬜ | `1` | 启用缓存（默认 1，省费用） |

### 1.2 形态 B：Azure OpenAI

| 字段 | 必填 | 示例 | 说明 |
|---|---|---|---|
| `MIDSCENE_USE_AZURE_OPENAI` | ✅ | `1` | 切换到 Azure 模式 |
| `AZURE_OPENAI_ENDPOINT` | ✅ | `https://my-resource.openai.azure.com` | Azure 资源端点（控制台 → Keys & Endpoint） |
| `AZURE_OPENAI_DEPLOYMENT` | ✅ | `gpt-4o-vision` | **Deployment 名**（不是 model 名！） |
| `AZURE_OPENAI_API_VERSION` | ✅ | `2024-08-01-preview` | API 版本 |
| `MIDSCENE_MODEL_API_KEY` | ✅ | `<azure-key>` | Azure 资源 Key |

> ⚠️ Azure 形态下 `MIDSCENE_MODEL_NAME` 不被使用；模型由 `AZURE_OPENAI_DEPLOYMENT` 指定。
> ⚠️ Deployment 名由你在 Azure OpenAI Studio 创建模型部署时自定义；与底层 model（gpt-4o）不同。

### 1.3 形态 C：中转 / 自托管（OneAPI / LiteLLM / 自建网关）

| 字段 | 必填 | 示例 | 说明 |
|---|---|---|---|
| `MIDSCENE_OPENAI_BASE_URL` | ✅ | `https://relay.example.com/v1` | 网关 base URL（必须含 `/v1`） |
| `MIDSCENE_MODEL_API_KEY` | ✅ | `<relay-token>` | 网关下发的 Token |
| `MIDSCENE_MODEL_NAME` | ✅ | `gpt-4o` | 网关支持的模型名 |

> ⚠️ 中转形态下需自行验证网关支持 vision 多模态；OneAPI 部分模型仅支持纯文本，会导致 WEB 用例红。
> 切回直连：删掉 `MIDSCENE_OPENAI_BASE_URL` 即可恢复 OpenAI 直连。

---

## 2. 本地配置（local profile）

```bash
# tests/scripts/env/.env.local（已在 .gitignore，可放真实 Key）
MIDSCENE_MODEL_API_KEY=<your-openai-key>
MIDSCENE_MODEL_NAME=gpt-4o
MIDSCENE_CACHE=1
# 中转形态再加：
# MIDSCENE_OPENAI_BASE_URL=https://relay.example.com/v1

# Android E2E（铁律 7）额外字段
ANDROID_APP_ID=com.voice.room.android.local.debug   # 与 build.gradle.kts: applicationId + .local + .debug 一致
ADB_DEVICE_ID=                                      # 留空 = 自动选第一台已连接的真机/模拟器
```

```bash
npm run e2e:local            # 全量（Web + Android + 跨端 E2E）
npm run e2e:android          # 仅 Android（待落 npm script，见铁律 7 §7.4）
```

> Key 缺失时 Web/Android 套件**均**自动 skip（reason: `[MIDSCENE] api key missing — skipped`），不会红条。

---

## 2.A Android 形态接入要点

> 落实"E2E 完全使用 Midscene"铁律 7。Android E2E 走 `@midscene/android` 的 `agentFromAdbDevice`，纯 ADB 通道，不依赖 Appium。

### 2.A.1 前置依赖

| 依赖 | 验证命令 | 期望 |
|------|---------|------|
| ADB 已安装 | `adb version` | ≥ 1.0.41 |
| 真机/模拟器 已连接 | `adb devices` | 至少 1 行 `<id>\tdevice` |
| App 已安装 | `adb shell pm list packages \| grep ${ANDROID_APP_ID}` | 命中 |
| `@midscene/android` | `npm ls @midscene/android` | 已装（首次需 `npm i -D @midscene/android`） |

### 2.A.2 用例最小骨架

```ts
// tests/scripts/AND/TC-AUTH.spec.ts
import { test, expect } from '@playwright/test';
import { agentFromAdbDevice } from '@midscene/android';
import { execSync } from 'child_process';

test('TC-AUTH-00003: 新用户登录闭环', async () => {
  const agent = await agentFromAdbDevice(process.env.ADB_DEVICE_ID, {
    androidAdbPath: 'adb',
  });
  await agent.launchApp(process.env.ANDROID_APP_ID!);

  await agent.aiInput('+966500000900', '手机号输入框');
  await agent.aiTap('"获取验证码" 按钮');
  await agent.aiAssert('按钮文案变为 "60s 后重发"');

  // 副作用断言：AppServer 真的收到了请求（铁律 6）
  await test.step('AppServer 收到 verification-codes', () => {
    const log = execSync('tail -n 100 ~/.voiceroom/server.log').toString();
    expect(log).toMatch(/POST \/api\/v1\/auth\/verification-codes.*200/);
  });
});
```

### 2.A.3 Android 端常见坑

| 现象 | 排查 |
|------|------|
| `agentFromAdbDevice` 抛 `no devices` | `adb devices` 是否有设备；多设备需显式传 `ADB_DEVICE_ID` |
| `aiTap` 命中错位 | 屏幕缩放 / 软键盘遮挡；先 `agent.aiKeyboardPress('back')` 收键盘再操作 |
| `launchApp` 启动空白 | `ANDROID_APP_ID` 与 `applicationId + flavorSuffix + buildTypeSuffix` 不一致（本仓库 local+debug 是 `com.voice.room.android.local.debug`） |
| 视觉模型回 "I cannot see images" | 中转网关不支持 vision，切回直连或换 deployment |

---

## 3. CI Secret 注入（GitHub Actions）

```yaml
# .github/workflows/e2e.yml（节选 — 严禁明文 Key）
name: E2E
on: [push, pull_request]

jobs:
  e2e-staging:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npx playwright install --with-deps chromium
      - name: Run E2E (staging)
        env:
          E2E_PROFILE: staging
          # ⚠️ 仅引用 Secret，严禁明文
          MIDSCENE_MODEL_API_KEY: ${{ secrets.MIDSCENE_MODEL_API_KEY }}
          MIDSCENE_MODEL_NAME: gpt-4o
          MIDSCENE_OPENAI_BASE_URL: ${{ vars.MIDSCENE_OPENAI_BASE_URL }}
          MIDSCENE_CACHE: '1'
          E2E_VALID_TOKEN: ${{ secrets.E2E_VALID_TOKEN }}
        run: npm run e2e:staging
```

### 3.1 安全规约（红线）

| # | 规约 | 是否强制 |
|---|------|---------|
| 1 | 必须用 `${{ secrets.MIDSCENE_MODEL_API_KEY }}` 引用，不得内联 Key | ✅ |
| 2 | 禁止 `echo $MIDSCENE_MODEL_API_KEY` / `printenv` / `set -x` 暴露 | ✅ |
| 3 | 非敏感字段（如 base URL）可用 `${{ vars.* }}`，仅 Key 走 `secrets` | ✅ |
| 4 | `.e2e-runtime.json` 永不落盘 API Key（`envLoader.sanitizeEnvForRuntimeJson` 兜底） | ✅ |
| 5 | error 消息中字段值须以 `***` 替代（envLoader 已实现） | ✅ |

> 反例（**严禁** 提交至仓库）：以 yaml 直接内联 Key 值（`MIDSCENE_MODEL_API_KEY` 后接 `sk-` 开头明文）会被 push protection / secret scanning 拦截，必须用 `${{ secrets.* }}` 引用。

---

## 4. 缺 Key 自动 skip 策略

`tests/scripts/support/fixtures.ts` 注册了 `midsceneReady` auto fixture：

| 触发条件 | 行为 |
|---------|------|
| spec 路径含 `tests/scripts/WEB/` 且 `MIDSCENE_MODEL_API_KEY` 为空 | `test.skip(true, '[MIDSCENE] api key missing — skipped')` |
| 非 WEB spec（API / ADMIN_WEB / APPSERVER） | 不 skip（不依赖 Midscene） |
| WEB spec 且 Key 存在 | 正常运行 |

### 4.1 协同矩阵（与 prodSafeGuard）

| profile | @prod-safe | WEB spec | apiKey 空 | 行为 |
|---|---|---|---|---|
| local | — | 是 | 是 | **skip**（缺 Key） |
| local | — | 是 | 否 | 跑 |
| staging | — | 是 | 是 | **skip**（缺 Key） |
| prod | 否 | 是 | 任意 | **skip**（prod-safe 先行） |
| prod | 是 | 是 | 否 | 跑 |

---

## 5. 限流与回退

| 场景 | 表现 | 处理 |
|------|------|------|
| 401 Unauthorized | SDK 抛错 | Key 失效或 Azure deployment 名错（见 FAQ §6.2） |
| 429 Rate Limit | SDK 抛错或自动重试 | 启用 `MIDSCENE_CACHE=1`；CI 串行执行 WEB 套件 |
| 网关超时 | playwright timeout（默认 60s） | `playwright.config.ts` 设 `retries: process.env.CI ? 1 : 0` |
| 中转网关挂 | 持续 5xx/401 | 临时 unset `MIDSCENE_OPENAI_BASE_URL` 切回直连 |
| Key 配额耗尽 | 持续 429 | **不**自动 skip；手工置空 `MIDSCENE_MODEL_API_KEY` 重跑 → WEB 全 skip 逃生 |

---

## 6. FAQ / 常见问题

### 6.1 我的 WEB 用例全部 skip 了，原因？
查 Playwright HTML 报告 skip reason：
- `[MIDSCENE] api key missing — skipped` → 检查 `MIDSCENE_MODEL_API_KEY`（本地 `.env.local` / CI Secret）是否注入；
- `prod profile only runs tests tagged @prod-safe` → 改打 `@prod-safe` 标签或换 staging。

### 6.2 401 Unauthorized — Azure deployment 名误填
Azure 形态下 `AZURE_OPENAI_DEPLOYMENT` 必须填**部署名**（在 Azure OpenAI Studio → Deployments 列表里的「Deployment name」列），**不是** model 名（如 `gpt-4o`）。
- 路径：Azure Portal → 你的 OpenAI 资源 → Resource Management → Model deployments → 点击进入 Studio → Deployments → 看 "Deployment name" 列。
- 填 model 名（`gpt-4o`）或不存在的部署名都会回 `401` 或 `404 DeploymentNotFound`。

### 6.3 429 Rate Limit — 限流 / 配额耗尽
- 短期：保留 `MIDSCENE_CACHE=1`（默认）让重复 prompt 走缓存；
- 中期：CI 中将 WEB 套件设为 `workers: 1` 串行；
- 长期：升级模型配额或切到企业 Azure 资源。
- 如果 Key 配额耗尽且短期不能恢复：**临时 unset `MIDSCENE_MODEL_API_KEY`** 让 WEB 用例自动 skip，避免阻塞主流水线（注意：仅在 RUNBOOK 授权下使用，避免静默回归）。

### 6.4 timeout / 超时 — 网关响应慢
- 默认 playwright `timeout: 60s`；视觉模型大图理解可能 30s+；
- 提升 spec 内 `test.setTimeout(120_000)`；
- CI 启用 `retries: 1` 容忍偶发网络抖动；
- 中转网关延迟过高时，考虑切回 OpenAI 直连（删 `MIDSCENE_OPENAI_BASE_URL`）。

### 6.5 中转网关 vision 不支持
OneAPI / LiteLLM 部分模型适配器仅转发文本 chat，不支持 multipart vision 输入。表现：调用回 400 / 模型回 "I cannot see images"。
- 验证：`curl https://<relay>/v1/models | jq '.data[] | select(.id=="gpt-4o")'`；
- 解决：换支持 vision 的模型，或回退 OpenAI 直连。

### 6.6 我能在日志里看到 Key 吗？
**不能**。
- envLoader error 消息以 `***` 替代实值；
- `.e2e-runtime.json` 由 `sanitizeEnvForRuntimeJson` 脱敏（apiKey 字段写 `''`）；
- worker 端通过 `process.env.MIDSCENE_MODEL_API_KEY`（writeProcessEnv 注入）读取真实 Key，runtime json 仅作其他字段桥接。
- 若发现日志泄漏，立即 rotate Key + 提 Issue。

---

## 7. 验证清单

```bash
# 本地：填好 .env.local 后
npm run e2e:local -- tests/scripts/WEB/TC-AUTH.spec.ts

# 验证 fixture skip（清空 Key）
MIDSCENE_MODEL_API_KEY= npx playwright test tests/scripts/WEB/TC-AUTH.spec.ts
# → 期望：全部用例 skip，reason: [MIDSCENE] api key missing — skipped

# 验证 runtime json 不泄漏
grep -E 'sk-[A-Za-z0-9]{20,}' tests/scripts/.e2e-runtime.json
# → 期望：无输出

# 单测
npx playwright test --config=playwright.unit.config.ts tests/scripts/support/__tests__/midsceneSkip.test.ts
```
