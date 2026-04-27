# 全局代码审查报告: 模块9 E2E 测试基建 (E2E QA Foundation)
> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [1/10]

---

## 0. 流转规则
- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由[GlobalReview]进行全局代码审查
- [GlobalReview]审查通过，则修改负责人 [-] 状态 [✅ Passed]
- [GlobalReview]审查未通过，则修改负责人 [TDD] 状态 [❌ Failed], 并将审查意见填入文档下方
- 处于负责人 [TDD] 状态 [❌ Failed]，则由[TDD]根据审查意见进行代码修复并自测
- [TDD]修复之后，将状态改为负责人 [GlobalReview] 状态 [⏳ In Review]

---

## 1. 审查上下文

- **审查范围**：模块 9 E2E 测试基建（E2E QA Foundation）整体架构与基建可用性审查
- **包含任务模块**：[模块 9: E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)
- **包含任务**：T-0000E、T-0000F、T-0000G、T-0000H、T-0000I、T-0000J、T-0000K、T-0000L、T-00040、T-10020、T-20020、T-30050（共 12 个）
- **关联 TDS**：
  - 基建侧：[T-0000E](../tds/infra/T-0000E.md)（多环境切换器总设计）、[T-0000F](../tds/infra/T-0000F.md)（.env 模板）、[T-0000G](../tds/infra/T-0000G.md)（Seed/Reset/Preflight 三件套）、[T-0000H](../tds/infra/T-0000H.md)（globalSetup/Teardown/envLoader）、[T-0000I](../tds/infra/T-0000I.md)（npm scripts 一键命令）、[T-0000J](../tds/infra/T-0000J.md)（baseURL/@prod-safe 修复）、[T-0000K](../tds/infra/T-0000K.md)（Midscene LLM 配置）、[T-0000L](../tds/infra/T-0000L.md)（E2E_RUNBOOK 启动 SOP）
  - 各端配置：[T-00040](../tds/server/T-00040.md)（AppServer staging.toml）、[T-10020](../tds/adminServer/T-10020.md)（AdminServer config/）、[T-20020](../tds/web/T-20020.md)（Web 多 profile env）、[T-30050](../tds/android/T-30050.md)（Android productFlavors）
- **开始时间**：2026-04-27

---

## 2. 审查关切（来自 PO/协调者）

本批次为**架构级整体审查**，不重复审已 Passed 任务的实现细节，重点回答以下三个核心问题，确保模块9作为 Phase 1.6 测试基建的对外承诺已兑现：

### 关切 ①：能否快速进行 E2E 测试？
- 新人从 `git clone` 到第一条 smoke 用例全绿，是否真的 5 分钟可达？
- `npm run e2e:local` / `e2e:staging` / `e2e:prod` 链路是否端到端可执行（含 globalSetup → preflight → seed → 跑测 → teardown）？
- preflight 五端探活（AppServer、AdminServer、Web、Postgres、Redis 或等价物）是否真实生效，失败时报错是否清晰可定位？
- Midscene LLM Key 缺失时 Web 用例是否优雅 skip 而非 fail？

### 关切 ②：是否具备"一键部署服务 + 一键启动测试"的能力？
- 是否存在 `npm run e2e:up`（或等价命令）一键拉起 docker-compose 全栈（AppServer + AdminServer + Web + Postgres + Redis）？
- 一键命令是否覆盖：拉起服务 → 等待健康 → seed 数据 → 跑 E2E → 失败保留现场 → 成功 teardown？
- 三档 profile（local/staging/prod）切换是否真的"零改代码"，仅靠环境变量切换？
- `@prod-safe` 标签机制是否能阻止破坏性用例误跑到 prod？

### 关切 ③：环境变量是否能自动写入 Android 与 AdminWeb（跨端注入能力）？
- **AdminWeb 侧**：Vite `import.meta.env.VITE_ADMIN_API_BASE_URL` 等变量是否真的从根 `.env.{profile}` 自动注入？切换 profile 是否需要手动修改 web 子项目的 .env？
- **Android 侧**：`productFlavors {local/staging/prod}` 是否真的将 `BASE_URL` / `WS_URL` / `applicationIdSuffix` 等通过 BuildConfig 注入？三档 APK 是否能并存安装？
- 是否存在统一的 env 单一事实源（根 `.env.{profile}`），还是各端各自维护一份（违反单一事实源原则）？
- envLoader 是否能在 E2E 启动时把同一份配置同时投喂给 Playwright（baseURL）、Midscene（API Key）、Android 测试（adb 注入或 BuildConfig）、AdminWeb（VITE_*）？

### 关切 ④（附加）：基建本身的健壮性
- Seed/Reset 脚本是否幂等？反复跑不会污染数据库？
- globalTeardown 失败时是否会泄漏端口/容器/进程？
- CI 接入 SOP 是否完整（Secret 注入、产物上传、失败重试策略）？
- 文档（E2E_RUNBOOK.md、MIDSCENE_SETUP.md）是否与代码实际行为一致，没有"文档说能跑实际跑不通"的情况？

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**

#### 一、四个核心关切的逐条结论

| # | 关切 | 结论 | 关键证据 |
|---|---|---|---|
| ① | 5 分钟冷启动 | ⚠️ **部分兑现** | `doc/tests/E2E_RUNBOOK.md:21,29-51` 把 “服务起齐 ≈ 1min” 计入 5min 预算，但实际 Step 4 要求 `cargo run -p server` / `cargo run -p admin-server` 冷编译整个 Rust 工作区（首跑分钟级，远超 1min）。新人在干净 clone 上首次 5min 内极难跑通首条用例。preflight 5 端探活实现正确（`scripts/dev/preflight.sh:79-187` 含 fail-fast、退出码 11~15、彩色与 hint），Midscene 缺 Key 时 WEB 子树 `test.skip` 行为正确（`tests/scripts/support/fixtures.ts:45-62`）。 |
| ② | 一键部署+一键测试 | ❌ **未兑现** | `docker-compose.yml` 仅声明 `postgres` + `redis` 两个 service（共 41 行），完全不含 AppServer / AdminServer / Web。仓库**不存在** `npm run e2e:up`，`package.json:scripts`（10-15 行）只冻结 6 条命令；RUNBOOK §2 Step 4（行 42-45）明确要求新人开 3 个终端各跑 `cargo run` / `npm run dev`，与 PO 期望的「一键拉起全栈」完全相反。`@prod-safe` 三道防线已落地（`playwright.config.ts:30` config grep + `package.json:12` CLI `--grep "@prod-safe"` + `tests/scripts/support/fixtures.ts:26-33` fixture L3），✅ 这部分兑现。 |
| ③ | 跨端环境变量自动注入 / 单一事实源 | ❌ **未兑现** | (a) **AdminWeb 侧**：`app/web/.env.{development,staging,production,test}` 5 个文件独立维护，URL 全部硬编码（`stg-app.example.com` 等），与根 `tests/scripts/env/.env.staging.example` 没有任何机制保证同步。`app/web/vite.config.ts` 未配置任何指向根 `.env` 的 envDir / loadEnv，Vite 默认仅读 `app/web/.env.<MODE>`。根 `.env.example:79-81` 注释明确写「VITE_* 收口于 app/web/.env.*，不在根 .env 中声明」——这是**有意识的双源设计**，但与 PO 「单一事实源」诉求冲突。(b) **Android 侧**：`app/android/app/build.gradle.kts:108-134` 中 staging / prod productFlavors 的 `API_BASE_URL` / `WS_URL` / `ANALYTICS_ENDPOINT` 全部**字面硬编码**，没有从 root `.env.staging` 读取的通道。仅 local flavor 通过 `local.properties` / env 变量回退（行 92-103），且环境变量名 `VOICE_ROOM_API_BASE_URL` 与 root `.env` 字段 `APP_SERVER_BASE_URL` **名称不一致**，envLoader（`tests/scripts/support/envLoader.ts:252-283`）也没有桥接。(c) envLoader → Playwright/Midscene 链路 ✅（`writeProcessEnv` 完整、`sanitizeEnvForRuntimeJson` 脱敏 API Key）。 |
| ④ | 基建健壮性 | ⚠️ **部分兑现** | ✅ Seed 幂等：`scripts/dev/seed-e2e.sql` 全表 `ON CONFLICT ... DO UPDATE` + UUIDv5 确定性 ID（`scripts/dev/seed-e2e.sh:69-92`）；✅ Teardown 仅 local 触发 reset 且失败仅 warn（`tests/scripts/support/globalTeardown.ts:26-54`）。❌ **CI 永红风险**：`.github/workflows/playwright.yml:7-22` 在 `pull_request`/`push` 上直接 `npx playwright test`，不启动任何依赖服务，也无 `CI_E2E_READY` / `E2E_PROFILE` 门禁；`globalSetup` 必走 preflight → 退码 11~15 → 流水线全红。⚠️ RUNBOOK §2 Step 5（行 49）建议冷启首跑 `npm run e2e:prod-smoke -- --list`，但 Step 2 只复制了 `.env.local`，未复制 `.env.prod` → envLoader 在 prod profile 下因缺 7 项 token 必抛 `MissingEnvError`（exit 78），与文档「首条 smoke 30s 验证链路」的承诺矛盾。 |

#### 二、缺陷清单（按 P0/P1/P2 分级）

- [ ] **缺陷 1**：[级别 P1] **CI 工作流 `playwright.yml` 永红——未起依赖服务即跑 Playwright**
  - **文件与行号**：`.github/workflows/playwright.yml:7-22`
  - **问题说明**：CI 在 `push` / `pull_request` 上直接执行 `npx playwright test`，但 `playwright.config.ts:26-27` 强制 `globalSetup`，后者会调用 `scripts/dev/preflight.sh` 探活 Postgres/Redis/AppServer/AdminServer/Web 五端；CI runner 上这五个服务一个都没起，必触退码 11~15，整个流水线必红。同时 `.github/workflows/ci.yml` 完全不跑 E2E，意味着 E2E 在 CI 上要么永红、要么不跑——与 §6 “CI 引用”描述的“Secret 注入 + 跑测”SOP 严重不符，且违反关切 ④「CI 接入 SOP 完整」红线。
  - **修复建议**：二选一——(a) 将 `playwright.yml` 改为 `workflow_dispatch` 手动触发，并在 job 中加 `services: postgres / redis` + `cargo run -p server &` + `npm --prefix app/web run dev &` + `npm run db:seed`，或直接 docker-compose up；(b) 在 `globalSetup` 入口加 `if (process.env.CI === 'true' && process.env.CI_E2E_READY !== '1') return;` 的软门禁（与 `.env.example:73` 的 `CI_E2E_READY=0` 注释承诺呼应），并在 `playwright.yml` 默认不设该 secret，让 CI 退化为编译/lint 期校验；选 (a) 为长期方案。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 2**：[级别 P1] **Android staging/prod productFlavors URL 硬编码，跨端单一事实源破裂**
  - **文件与行号**：`app/android/app/build.gradle.kts:108-134`
  - **问题说明**：staging / prod 两个 flavor 内 `API_BASE_URL` / `WS_URL` / `ANALYTICS_ENDPOINT` 全部字面硬编码 `https://stg-api.example.com/api` 等占位域名，**没有任何通道**从 root `.env.staging` / `.env.prod` 或 envLoader 注入；切换环境必须改 Kotlin 代码并重新编译，与 PO 关切 ③「productFlavors 真的将 BASE_URL/WS_URL 通过 BuildConfig 注入」「切换 profile 零改代码」直接冲突。同时 local flavor 读取的 env 名是 `VOICE_ROOM_API_BASE_URL` / `VOICE_ROOM_WS_URL`，与根 `.env` 字段 `APP_SERVER_BASE_URL` / `APP_WS_URL` 命名错位，envLoader (`tests/scripts/support/envLoader.ts:252-283`) 也没有桥接转发。
  - **修复建议**：(a) staging / prod flavor 改为 `resolveConfigValue(localProperties, "voiceRoomApiBaseUrl", "VOICE_ROOM_API_BASE_URL", "<占位>")` 同 local 写法；(b) 在 `envLoader.writeProcessEnv` 末尾追加桥接：`process.env.VOICE_ROOM_API_BASE_URL = env.appServerBaseUrl; process.env.VOICE_ROOM_WS_URL = env.appWsUrl;`；(c) 或在 `tests/scripts/AND` 启动脚本里把根 .env 主字段映射为 `gradlew -PvoiceRoomApiBaseUrl=...` 命令行属性。任一方案需在 RUNBOOK 增补 §「Android E2E 注入路径」段说明。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 3**：[级别 P1] **AdminWeb env 双源维护，根 `.env.{profile}` 切换不会传导到 Vite**
  - **文件与行号**：`app/web/.env.development` `app/web/.env.staging` `app/web/.env.production` `app/web/vite.config.ts:1-19`、根 `.env.example:79-81`
  - **问题说明**：4 个 web 端 env 文件**各自硬编码** `https://stg-app.example.com/api` 等 URL；vite.config.ts 没有 `envDir: '..'` 也没有 `loadEnv` 自定义，Vite 仅会从 `app/web/.env.<MODE>` 读取——这意味着将来 SRE 修改 `tests/scripts/env/.env.staging` 的 `ADMIN_WEB_URL` / 后端域名后，AdminWeb 构建产物**完全不会同步**。根 `.env.example` 注释（79-81）虽承认这是有意识设计，但仍违反 PO 关切 ③「单一事实源」红线。
  - **修复建议**：(a) 推荐方案——在 `app/web/vite.config.ts` 中接 `loadEnv(mode, path.resolve(__dirname, '../../tests/scripts/env'), 'VITE_')` 或 `'APP_/ADMIN_'` 前缀，并通过 `define` 把根 `.env.<profile>` 的 `APP_SERVER_BASE_URL` / `ADMIN_SERVER_BASE_URL` 注入为 `import.meta.env.VITE_*`；(b) 退而求其次——把 `app/web/.env.*` 文件全部清空 URL，改用 envLoader 在 globalSetup 阶段写出符号链接 / 生成临时 .env；(c) 至少在 RUNBOOK §3 + 根 `.env.example:79-81` 注释中**显著标注此为已知双源**并写明同步责任人，避免误导。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 4**：[级别 P2] **RUNBOOK 5min 预算未计入首次 cargo 冷编译**
  - **文件与行号**：`doc/tests/E2E_RUNBOOK.md:21,42-44`
  - **问题说明**：§1 时长预算把「服务起齐 ≈ 1min」计入 5min 总预算，但 Step 4 要求新人首次 `cargo run -p server` 与 `cargo run -p admin-server`，干净 clone 在普通 macOS / Linux 笔记本上首次编译 Rust 工作区通常 5~15 分钟，绝无 1min 可能。承诺与现实差距≥3 倍。
  - **修复建议**：把预算改为 `npm install ≈ 2min + cargo build (首次) 5~15min + 服务起齐 ≈ 30s + smoke ≈ 30s` ≈ **首次 8~18min / 复跑 ≤ 5min**；或在 §1 增加「先 `cargo build --workspace` 预热」前置步骤把编译耗时移到“准备”而非“5 步冷启动”阶段。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 5**：[级别 P2] **RUNBOOK 冷启 Step 5 推荐命令与已复制 env 不匹配**
  - **文件与行号**：`doc/tests/E2E_RUNBOOK.md:36-50`
  - **问题说明**：Step 2 只让用户复制 `.env.local`，Step 5 却推荐 `npm run e2e:prod-smoke -- --list` 作为「首次推荐 smoke 子集」。但 `e2e:prod-smoke` 走 `E2E_PROFILE=prod`，envLoader (`tests/scripts/support/envLoader.ts:163-170`) 在 profile≠local 时强制要求 7 项 token + 3 项 ID 必填，而冷启路径未提供 `.env.prod` → 必然 `MissingEnvError` 退码 78，新人首次按 RUNBOOK 操作会卡死在第一条命令上。
  - **修复建议**：把 Step 5 「smoke 子集」示例改为 `npm run e2e:local -- --list` 或 `npm run e2e:local -- --grep "@prod-safe"`（若 local profile 也命中 @prod-safe 子集），并把 `e2e:prod-smoke` 的演示移到 §5 远端凭据流程。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 6**：[级别 P2] **docker-compose 不含业务服务，与「一键起全栈」期望落差未在 RUNBOOK 明示**
  - **文件与行号**：`docker-compose.yml:1-41`、`doc/tests/E2E_RUNBOOK.md:39-45`
  - **问题说明**：`docker-compose.yml` 仅声明 postgres / redis 两个 service。这本身是合法设计选择（业务服务用 cargo / vite 本地起便于热重载），但 RUNBOOK §2 没有任何提示「本仓库不打算 docker 化业务服务」，新人按 PO 表述期望「`docker compose up` 拉起全栈」必然失望，且 §1 表格中 docker 自检命令也容易给人误导。建议在 §1 顶部用一行小字明确 "docker-compose 仅托管 PG/Redis，业务服务 dev 期一律 cargo / vite 本地起"。
  - **修复建议**：在 RUNBOOK §2 起始位置加一段「设计取舍」说明；或新增 `npm run e2e:up` 脚本聚合 `docker compose up -d postgres redis && cargo run -p server & cargo run -p admin-server & (cd app/web && npm run dev &) && wait-on http://localhost:3000/health http://localhost:3001/health http://localhost:5173`，让一键起栈成为真选项。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

#### 三、本轮总结

**本轮结论**: ❌ 存在 P1 级别问题（关切 ② 未兑现 / 关切 ③ 跨端单一事实源破裂 / 关切 ④ CI 永红），关切 ① 部分兑现存在文档与现实预算偏差。

**已落地优点（保留）**：preflight 实现严谨（fail-fast + 退码 11~15 + hint）；envLoader 类型安全 + 脱敏 .e2e-runtime.json；@prod-safe 三道防线（CLI grep + config grep + fixture L3）冗余度足够；seed 幂等且不写敏感字段；teardown local-only 且失败 non-fatal；fixtures 对 WEB 子树 Midscene 缺 Key 自动 skip 行为正确；envLoader 单测覆盖路径完整。

*(请在文档头部将状态机修改为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`，已同步修改)*

---

### 【第 2 轮审查】（PO 确认状态修正后重新 review）
**@GlobalReview 审查意见：**

#### 一、第 1 轮 6 个缺陷的代码现状逐条核对

| # | 缺陷 | 第 1 轮证据 | 第 2 轮实地核验 | 现状 |
|---|---|---|---|---|
| 1 (P1) | CI `playwright.yml` 永红 | 触发于 push/pull_request 直接 `npx playwright test`，无依赖服务 | `.github/workflows/playwright.yml:2-21` 触发器仍为 `push: [main, master] / pull_request: [main, master]`，job steps 仅 `npm ci → playwright install → npx playwright test`，**未** `services: postgres/redis`，**未** 启动 cargo / vite，**未** 设置 `CI_E2E_READY` 软门禁 | ❌ 未修复 |
| 2 (P1) | Android staging/prod 硬编码 + env 命名错位 | build.gradle.kts:108-134 | `app/android/app/build.gradle.kts:114-133` staging/prod flavor 仍直写 `"https://stg-api.example.com/api"` / `"https://api.example.com/api"` 等字面值，**无** `resolveConfigValue` 通道；envLoader 也未追加 `VOICE_ROOM_API_BASE_URL` 桥接（`tests/scripts/support/envLoader.ts` 无新增 export） | ❌ 未修复 |
| 3 (P1) | AdminWeb env 双源 | app/web/.env.* 4 文件 + vite.config.ts 不读根 .env | `app/web/.env.{development,staging,production,test}` 4 个文件依然存在；`app/web/vite.config.ts` 全文 17 行只含 `react()` plugin + vitest 配置，**无** `envDir`、**无** `loadEnv`、**无** `define` 注入根 .env；根 `.env.example:79-81` 双源注释也未更新为同步责任声明 | ❌ 未修复 |
| 4 (P2) | RUNBOOK 5min 预算未计 cargo 冷编译 | E2E_RUNBOOK.md:21 | `doc/tests/E2E_RUNBOOK.md:21` 仍写 `env/docker/服务起齐 ≈ 1min … ⇒ ≤ 5 分钟`，未追加 cargo build 预热段或修订预算口径 | ❌ 未修复 |
| 5 (P2) | RUNBOOK Step 5 推荐命令与已复制 env 不匹配 | E2E_RUNBOOK.md:36-50 | `doc/tests/E2E_RUNBOOK.md:36-50` Step 2 仍仅 `cp .env.local.example`；Step 5 第 49 行仍是 `npm run e2e:prod-smoke -- --list`，envLoader prod 必抛 MissingEnvError | ❌ 未修复 |
| 6 (P2) | docker-compose 不含业务服务，RUNBOOK 未明示 | docker-compose.yml:1-41 + RUNBOOK §2 | `docker-compose.yml` 仍只有 postgres + redis 两 service（41 行未变）；RUNBOOK §2 起始位置未补「设计取舍」段落，也未新增 `npm run e2e:up` 聚合命令（`package.json:scripts` 未变） | ❌ 未修复 |

> **复核方法**：直读源文件行号 + grep 关键变量名（`VOICE_ROOM_API_BASE_URL`、`envDir`、`loadEnv`、`CI_E2E_READY`、`e2e:up`），均无任何匹配变更。**TDD 本轮内未提交任何修复代码**——本次 review 系 PO 修正主表状态机后的程序性复核，6 个缺陷悉数维持原级别原描述，无需重新出具修复建议（参见第 1 轮对应条目）。

#### 二、四个 PO 关切的复核结论（结论维持第 1 轮，证据未变）

| # | 关切 | 第 2 轮结论 | 关键证据 |
|---|---|---|---|
| ① | 5min 冷启动 | ⚠️ **部分兑现**（同第 1 轮） | 缺陷 4 + 缺陷 5 未修，新人首次 5min 内仍极难达成首条 smoke 全绿 |
| ② | 一键部署 + 一键测试 | ❌ **未兑现**（同第 1 轮） | 缺陷 6 未修，仍需 3 终端手动起 cargo / vite；`npm run e2e:up` 仍不存在 |
| ③ | 跨端 env 自动注入 / 单一事实源 | ❌ **未兑现**（同第 1 轮） | 缺陷 2 + 缺陷 3 未修，Android staging/prod 与 AdminWeb 全部维持双源 |
| ④ | 基建健壮性 | ⚠️ **部分兑现**（同第 1 轮） | 缺陷 1 未修，CI 仍永红风险；Seed 幂等 / Teardown non-fatal / @prod-safe 三道防线等优点维持 |

#### 三、新增缺陷扫描

本轮在 6 个缺陷代码路径周边重新扫描了 `playwright.config.ts` / `package.json:scripts` / `tests/scripts/support/envLoader.ts` / `globalSetup.ts` / `globalTeardown.ts` / `app/web/vite.config.ts` / Android build.gradle.kts / docker-compose.yml / `.env.example` / RUNBOOK，**未发现任何新增 P0/P1/P2 缺陷**，也未发现因 PO 修正主表状态机引入的新副作用。第 1 轮记录的「已落地优点」全部保持。

#### 四、本轮总结

**本轮结论**: ❌ 第 1 轮 P1×3 + P2×3 共 6 个缺陷在最新代码上**全部仍存在**，无任何 commit 修复痕迹；PO 修正状态机后理应进入 TDD 修复轮，故维持 Failed 状态、**不递增修复轮次计数**（仍记 1/10），等待 TDD 首次实质性提交后再进入第 3 轮复审。

*(请在文档头部将状态机维持为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`，已同步保持)*

---

### 【第 1 轮 TDD 修复】

> 负责人：TDD ｜ 修复轮次：1/10 ｜ 状态：⏳ 等待 [GlobalReview] 复审

#### 一、修复方法论

严格遵循 **Red → Green → Refactor**：每个缺陷先在单测套件补/调用例（Red），再改实现（Green），最后跑全量回归（Refactor 验证）。所有改动遵循「单一事实源 = 根 `.env.{profile}`」红线与「零硬编码」约束，Android/Web 端 0 回归（通过保留默认值与既有断言契约）。

#### 二、缺陷修复逐条记录

##### 缺陷 1 [P1] CI 永红 — 软门禁兜底（方案 b）

- **修改文件**：
  - `.github/workflows/playwright.yml`：触发器改为 `workflow_dispatch`，加输入 `e2e_ready`，job env 透传为 `CI_E2E_READY`
  - `tests/scripts/support/globalSetup.ts`：`runGlobalSetup` 入口加 `CI=true && CI_E2E_READY!=1` 早退分支
  - `tests/scripts/support/__tests__/globalSetup.test.ts`：+3 用例（CI=true 缺旗标早退 / CI=true+CI_E2E_READY=1 正常跑 / 本地不受影响）
- **自测**：`npx playwright test --config=playwright.unit.config.ts` → CI 软门禁 3 用例全绿

##### 缺陷 2 [P1] Android staging/prod 硬编码 + envLoader 桥接缺失

- **修改文件**：
  - `tests/scripts/support/envLoader.ts`：`writeProcessEnv` 末尾追加 4 个 Android 桥接键 `VOICE_ROOM_API_BASE_URL/WS_URL/ANALYTICS_ENDPOINT/ENVIRONMENT`
  - `app/android/app/build.gradle.kts`：staging/prod flavor 改用 `resolveConfigValue(envKey, propKey, default)`，default 保留商店域名以保 0 回归
  - `tests/scripts/support/__tests__/envLoader.test.ts`：+1 桥接用例（writeProcessEnv 调用后 4 个 VOICE_ROOM_* 键齐全）
  - `tests/scripts/support/__tests__/androidGradle.test.ts`：新建 3 个静态守护用例（staging/prod 用 resolveConfigValue + envLoader 写入桥接键）
- **自测**：
  - 根 unit：4 用例全绿
  - `cd app/android && ./gradlew :app:testStagingDebugUnitTest :app:testProdDebugUnitTest :app:testLocalDebugUnitTest --rerun-tasks` → BUILD SUCCESSFUL（staging/prod/local 全绿，0 回归）

##### 缺陷 3 [P1] AdminWeb env 双源

- **修改文件**：
  - `app/web/vite.config.ts`：函数式 defineConfig，自实现 `parseDotenv`（无新依赖），从根 `tests/scripts/env/.env.{profile}.example` + 真值 `.env.{profile}` 读取，通过 `define` 注入 4 个 `import.meta.env.VITE_*`；ESM 兼容用 `fileURLToPath(import.meta.url)` 兜底 `__dirname`
  - `app/web/.env.{development,staging,production,test}`：清空 URL 字段（仅留注释），消除双源
  - `.env.example`：第 79-86 行注释更新跨端字段语义（含 Android 桥接说明 + Web define 说明）
  - `app/web/src/core/config/envFiles.test.ts`（T-20020）：U2.2/U5.1/U6.2 改为新单一事实源契约（不含字面 URL / vite.config define 注入校验 / 根 env 真源校验）
  - `tests/scripts/support/__tests__/viteConfig.test.ts`：新建 3 用例（vite.config 引用根 env / 子项目 env 不含 URL / mode=staging 工厂注入正确 VITE_*）
- **自测**：
  - 根 unit：3 用例全绿
  - `cd app/web && npm test` → **517 / 517 passed**（含 envFiles.test.ts 全部用例）

##### 缺陷 4 [P2] RUNBOOK 时长预算不实

- **修改文件**：`doc/tests/E2E_RUNBOOK.md` §1 时长预算改为「首次 8~18min（含 cargo build）/ 复跑 ≤5min」
- **自测**：`runbook.test.ts` U-12 守护 cargo 预热条目，全绿

##### 缺陷 5 [P2] docker-compose 设计取舍未文档化

- **修改文件**：`doc/tests/E2E_RUNBOOK.md` §2 加「设计取舍」段：docker 仅起 Postgres+Redis，cargo/vite 走宿主机加速热重载与排错；Step 5 改用 `e2e:local --list` / `--grep "@prod-safe"` 两条命令
- **自测**：`runbook.test.ts` U-13/U-14 守护设计取舍 + Step 5 命令矩阵，全绿

##### 缺陷 6 [P2] 缺一键起栈命令

- **修改文件**：
  - `package.json`：scripts 加 `"e2e:up": "bash scripts/dev/e2e-up.sh"`
  - `scripts/dev/e2e-up.sh`：新建（docker compose up -d → 3 个 cargo/vite 后台进程 → `npx -y wait-on@^7` 等 5 端健康；PID 写 `.e2e-up.pids`）
  - `scripts/dev/e2e-down.sh`：新建（按 PID 停服 + docker compose down）
  - `doc/tests/E2E_RUNBOOK.md` §3 命令矩阵新增 `npm run e2e:up` 行
- **自测**：`runbook.test.ts` U-15/U-16 守护 e2e:up + Android 注入路径段，全绿

#### 三、回归测试矩阵

| 测试套件 | 命令 | 结果 |
|---|---|---|
| 根 Playwright unit | `npx playwright test --config=playwright.unit.config.ts` | ✅ 94 passed + 2 skipped |
| AdminWeb vitest | `cd app/web && npm test` | ✅ 517 passed |
| Android staging/prod/local unit | `cd app/android && ./gradlew :app:testStagingDebugUnitTest :app:testProdDebugUnitTest :app:testLocalDebugUnitTest --rerun-tasks` | ✅ BUILD SUCCESSFUL（0 回归） |

#### 四、本轮总结

**本轮结论**：6 个缺陷（P1×3 + P2×3）已逐条按 TDD 红绿循环修复，所有新增/修改测试用例全绿，AdminWeb 与 Android 既有单测 0 回归。状态机切回 `负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [1/10]`，等待第 3 轮复审。

---
