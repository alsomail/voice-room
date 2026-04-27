# 模块 9: E2E 测试基建 (E2E QA Foundation)

> 返回 [任务总索引](./index.md)

## Phase 1.6: 测试基建（与功能 Epic 解耦，独立交付）

> **背景**：截至 2026-04-26，全量功能模块（0~8）已具备较完整的单元测试与代码实现，但跨端 E2E 真跑链路存在 14 项关键缺口（详见 [T-0000E TDS](../tds/infra/T-0000E.md) 第一节体检结论）。本模块统一交付 **多环境（local / staging / prod）E2E 切换体系 + 健康预检 + Seed 数据 + 启动 SOP**，使 `tests/scripts/**` 真正可执行、可复现、可隔离。

> **总设计入口**：所有任务遵循 [T-0000E：E2E 多环境分层与切换器总设计](../tds/infra/T-0000E.md)（主 TDS），子任务 TDS 仅描述具体落地差异。

> **执行铁律**：
> 1. P0 任务（T-0000E/F/G/H）必须按依赖顺序串行完成，先让 `local` 跑通；
> 2. P1 任务（T-00040/T-10020/T-20020/T-30050）可并行；
> 3. P2/P3 任务（T-0000I/J/K/L）依赖前序完成。

---

## 模块 9: E2E 测试基建 (E2E QA Foundation)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-0000E** | 基建 | Infra/E2E | E2E 多环境分层与切换器总设计 [TDS](../tds/infra/T-0000E.md) | 无 | 输出多环境（local/staging/prod）总体设计：目录结构、配置加载链、切换机制、安全红线、Seed 契约、健康预检契约 | 1. TDS 文档完成度 100%（含字段表、迁移路径、风险矩阵）<br>2. 所有下游 11 个子任务的接口契约在本 TDS 内已冻结<br>3. 通过 Review | 3 | Dod | ✅ Done | ✅ Passed | - | ⏳ Pending |
| **T-0000F** | 基建 | Infra/E2E | 根 `.env.example` 修订 + 三档 profile 模板 [TDS](../tds/infra/T-0000F.md) | T-0000E | 1. 修复 `.env.example` 数据库密码 typo `app_server_pwd → app_server_pass`<br>2. 新增 `tests/scripts/env/.env.{local,staging,prod}.example`<br>3. 增加 `E2E_PROFILE` `E2E_ALLOW_WRITES` `VITE_ADMIN_API_BASE_URL` 字段 | 1. 三档 example 字段对齐 T-0000E 定义<br>2. 缺字段时 envLoader 抛出**显式启动错误**而非用例中途崩溃<br>3. `.gitignore` 覆盖所有真实 `.env` | 2 | Dod | ✅ Done | ✅ Passed | - | ⏳ Pending |
| **T-0000G** | 基建 | Infra/E2E | Seed/Reset/Preflight 三件套脚本 [TDS](../tds/infra/T-0000G.md) | T-0000E, T-0000A | 1. `scripts/dev/seed-e2e.sql` 幂等创建 E2E 测试用户/房间/Token<br>2. `scripts/dev/reset-e2e.sh` 幂等清空测试数据<br>3. `scripts/dev/preflight.sh` 5 端健康检查（PG/Redis/AppServer/AdminServer/Web） | 1. seed 重复运行结果一致（`ON CONFLICT DO UPDATE`）<br>2. preflight 任一服务挂时 2 秒内彩色定位<br>3. reset 不影响业务表结构，仅清测试数据 | 4 | DoD | ✅ Done | ✅ Passed | - | ⏳ Pending |
| **T-0000H** | 基建 | Infra/E2E | E2E `globalSetup`/`globalTeardown`/`envLoader` 三件套 [TDS](../tds/infra/T-0000H.md) | T-0000F, T-0000G | Playwright 启动前依据 `E2E_PROFILE` 加载 env、调 preflight、按需触发 seed；结束后调 reset；envLoader 是单一 env 加载源 | 1. `E2E_PROFILE=local` 全链路通过<br>2. `E2E_PROFILE=staging` 缺关键字段时抛 `MissingEnvError`<br>3. `prod` profile 默认 `E2E_ALLOW_WRITES=0`，写操作类用例自动 skip | 4 | DoD | In Progress | ✅ Passed | - | ⏳ Pending |
| **T-00040** | App Server | Config | AppServer config 补全 + 新增 staging.toml [TDS](../tds/server/T-00040.md) | T-0000E | 1. `config/default.toml` 补全 `[database] [redis] [jwt] [server.host]` 字段（值用 env 占位）<br>2. 新增 `staging.toml`<br>3. `dev/test/prod.toml` 字段对齐<br>4. 启动时强校验关键字段 | 1. 缺 JWT_SECRET/DATABASE_URL 时启动失败给明确错误<br>2. `APP_PROFILE=staging` 加载链正确<br>3. 现有测试 0 回归 | 3 | Plan | Todo | - | - | ⏳ Pending |
| **T-10020** | Admin Server | Config | AdminServer 引入 config/ 目录（与 server 对称） [TDS](../tds/adminServer/T-10020.md) | T-0000E | 1. 新建 `app/adminServer/config/{default,dev,test,staging,prod}.toml`<br>2. 引入 `config` crate<br>3. `main.rs` 由纯环境变量改为 `default + {profile} + env` 加载链 | 1. `ADMIN_PROFILE=staging` 加载链正确<br>2. 缺关键字段启动失败明确报错<br>3. 现有测试 0 回归 | 4 | Plan | Todo | - | - | ⏳ Pending |
| **T-20020** | Web | Config | Web 多 profile env + VITE_ADMIN_API_BASE_URL 收口 [TDS](../tds/web/T-20020.md) | T-0000E | 1. 新增 `.env.test` `.env.staging`<br>2. `.env.example` 补 `VITE_ADMIN_API_BASE_URL`<br>3. `vite.config.ts` 验证 `mode` 注入正确 | 1. `vite --mode staging` 加载 `.env.staging`<br>2. apiClient 默认值删除（强制走 env）<br>3. 现有 unit test 0 回归 | 2 | Plan | Todo | - | - | ⏳ Pending |
| **T-30050** | Android | Build | Android productFlavors {local/staging/prod} + 独立 applicationIdSuffix [TDS](../tds/android/T-30050.md) | T-0000E | 1. `build.gradle.kts` 新增 3 个 flavor 维度<br>2. 每 flavor 独立 `BuildConfig.API_BASE_URL/WS_URL/ANALYTICS_ENDPOINT`<br>3. `applicationIdSuffix`：`.local` / `.stg` / 无<br>4. local 才允许 `usesCleartextTraffic` | 1. 同设备能并存 `voiceroom.local` `voiceroom.stg` `voiceroom` 三包<br>2. staging/prod flavor 强制 HTTPS+WSS<br>3. `assembleLocalDebug` 通过 | 4 | Plan | Todo | - | - | ⏳ Pending |
| **T-0000I** | 基建 | Infra/E2E | `package.json` scripts 一键命令 [TDS](../tds/infra/T-0000I.md) | T-0000H | 新增 `e2e:local` `e2e:staging` `e2e:prod-smoke` `db:seed` `db:reset` `preflight` 6 个 script | 1. `npm run preflight` 1 秒内输出健康表<br>2. `npm run e2e:local` 等价 `E2E_PROFILE=local playwright test`<br>3. `npm run e2e:prod-smoke` 仅跑 `@prod-safe` 标签 | 1 | Plan | Todo | - | - | ⏳ Pending |
| **T-0000J** | 基建 | Infra/E2E | E2E 用例 baseURL/密码 typo 修复 + @prod-safe 标签 [TDS](../tds/infra/T-0000J.md) | T-0000H | 1. `playwright.config.ts` 由 envLoader 注入 `baseURL`<br>2. 全部用例的硬编码 DB 密码改读 env<br>3. read-only smoke 用例打 `@prod-safe` 标签 | 1. `grep -r 'app_server_pwd' tests/` 0 命中<br>2. WEB 用例可用 `page.goto('/login')` 相对路径<br>3. 至少 5 条 smoke 用例打标 | 2 | Plan | Todo | - | - | ⏳ Pending |
| **T-0000K** | 基建 | Infra/E2E | Midscene LLM 配置接入文档 + CI Secret 流程 [TDS](../tds/infra/T-0000K.md) | T-0000F | 输出 `doc/tests/MIDSCENE_SETUP.md`：本地 Key 注入、CI Secret 注入、限流与回退策略 | 1. 文档含三种部署形态（OpenAI 直连/Azure/中转）配置示例<br>2. CI workflow 引用 `MIDSCENE_MODEL_API_KEY` Secret 而非明文<br>3. Key 缺失时 WEB 用例 skip 而非 fail | 1 | Plan | Todo | - | - | ⏳ Pending |
| **T-0000L** | 基建 | Infra/E2E | E2E 启动 SOP（E2E_RUNBOOK.md） [TDS](../tds/infra/T-0000L.md) | T-0000I, T-0000J | 输出 `doc/tests/E2E_RUNBOOK.md`：三环境切换命令矩阵、常见故障排查表、CI 接入示例 | 1. 含 local 冷启动 5 分钟可跑通的 step-by-step<br>2. 故障排查表覆盖 preflight 5 端 × 常见故障<br>3. 含 staging 远端凭据获取流程占位 | 2 | Plan | Todo | - | - | ⏳ Pending |

**汇总**：12 个 Task，预估总工时 **32 人时（≈4 人天）**。

---

## 任务依赖图

```
T-0000E (主设计)
    │
    ├─→ T-0000F (env 模板)
    │       │
    │       ├─→ T-0000K (Midscene 文档)
    │       └─→ T-0000G (Seed/Reset/Preflight)
    │               │
    │               └─→ T-0000H (globalSetup/Teardown/envLoader)
    │                       │
    │                       ├─→ T-0000I (npm scripts)
    │                       │     │
    │                       │     └─→ T-0000L (RUNBOOK)
    │                       └─→ T-0000J (用例修复 + @prod-safe)
    │
    ├─→ T-00040 (AppServer config)    ┐
    ├─→ T-10020 (AdminServer config)  ├─ 可并行（P1）
    ├─→ T-20020 (Web 多 env)          │
    └─→ T-30050 (Android Flavor)      ┘
```

## 关键里程碑

| 里程碑 | 完成判定 | 价值 |
|--------|---------|------|
| **M1：本地 E2E 跑通** | T-0000E/F/G/H + T-0000J 完成 | 个人电脑可执行全部 35 个 E2E 用例 |
| **M2：多环境对称** | T-00040/T-10020/T-20020/T-30050 完成 | staging 远端凭据填入即可切换 |
| **M3：DX 与文档闭环** | T-0000I/K/L 完成 | 一键命令 + Runbook + Midscene 集成完整 |

