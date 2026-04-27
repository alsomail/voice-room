# 系统架构文档索引

> **原始文件**: `doc/ARCHITECTURE.md`（已拆分为本目录下的子文件）
> **拆分日期**: 2026-04-20

本目录包含实时语聊房项目的完整系统架构规范。按主题拆分为以下子文件，便于精准检索和增量更新。

---

## 📑 子文件索引

| # | 文件 | 内容概要 | 原章节 |
|---|------|---------|--------|
| 0 | [goals_and_overview.md](goals_and_overview.md) | 文档目标、四端技术栈、总体架构 Mermaid 图、分层原则 | §1-§2 |
| 1 | [directory_and_ddd.md](directory_and_ddd.md) | Monorepo 目录结构、目录总原则 | §3 |
| 2 | [domain_design.md](domain_design.md) | 业务域拆分、bounded context、模块结构、Rust 分层规范 | §4 |
| 3 | [android_architecture.md](android_architecture.md) | Android Clean Architecture + MVVM、关键接口 | §5 |
| 4 | [web_architecture.md](web_architecture.md) | Web Admin 后台架构定位与约束 | §6 |
| 5 | [api_and_auth.md](api_and_auth.md) | HTTP 统一返回体、JWT 鉴权、WS 鉴权与 Session 绑定 | §7 |
| 6 | [websocket_and_state.md](websocket_and_state.md) | WS 信令格式、房间状态同步、RoomStateRepository、幂等防重 | §8 |
| 7 | [transaction_and_gift.md](transaction_and_gift.md) | 送礼事务强一致性、事务边界、表结构、广播时机 | §9 |
| 8 | [anticorruption_layer.md](anticorruption_layer.md) | 客户端 & 服务端防腐层接口定义 | §10 |
| 9 | [resilience.md](resilience.md) | 弱网高可用：心跳、重连、乐观 UI、优雅降级、状态回补 | §11 |
| 10 | [observability.md](observability.md) | 结构化日志、客户端埋点防腐层、MENA 弱网上报、崩溃捕获、合规 | §12 |
| 11 | [mena_localization.md](mena_localization.md) | 中东本土化：i18n、RTL、时间与数字格式 | §13 |
| 12 | [code_standards.md](code_standards.md) | 各端 Lint/格式化规范、Git Hooks 与 CI | §14 |
| 13 | [environments_cicd.md](environments_cicd.md) | 多环境配置、CI/CD、Gateway、实施红线、落地优先级 | §15-§17 |

---

## 🔗 关联文档

- **协议契约**: [doc/protocol/index.md](../protocol/index.md)
- **产品需求**: [doc/product/index.md](../product/index.md)
- **任务看板**: [doc/tasks/index.md](../tasks/index.md)
- **各端实现架构**: `doc/arch/{server,adminServer,android,web}/index.md`
- **E2E 测试基建多环境切换**: [doc/tds/infra/T-0000E.md](../tds/infra/T-0000E.md) —— 多环境（local/staging/prod）分层切换、健康预检、Seed 数据、启动 SOP
- **三档 .env profile 模板**: [T-0000F TDS](../tds/infra/T-0000F.md) + [tests/scripts/env/](../../tests/scripts/env/) —— `.env.{local,staging,prod}.example` 字段表与契约
- **测试基建脚本三件套**: [T-0000G TDS](../tds/infra/T-0000G.md) + [`scripts/dev/`](../../scripts/dev/) —— Seed/Reset/Preflight 幂等脚本、sign-jwt CLI 工具（`app/shared/src/bin/sign_jwt.rs`）
- **E2E globalSetup/Teardown/envLoader**: [T-0000H TDS](../tds/infra/T-0000H.md) + [`tests/scripts/support/`](../../tests/scripts/support/) —— Playwright 启动期编排器（envLoader 单一加载源 + globalSetup 5 步 + globalTeardown 幂等清理 + fixtures 五道防线）
- **npm scripts 一键命令**: [T-0000I TDS](../tds/infra/T-0000I.md) + [`package.json` scripts](../../package.json) —— 6 个一键命令（`e2e:local/staging/prod-smoke` + `db:seed/reset` + `preflight`）、cross-env 跨平台注入、退出码透传契约、双引号 grep 防 Windows 单引号假绿
- **E2E 用例硬化与 @prod-safe 标签体系**: [T-0000J TDS](../tds/infra/T-0000J.md) + [`tests/scripts/support/__tests__/specHardening.test.ts`](../../tests/scripts/support/__tests__/specHardening.test.ts) —— `playwright.config.ts` 双 key fallback（`_E2E_RUNTIME_ADMIN_WEB_URL ?? ADMIN_WEB_URL`）消解时序风险；21 个 spec 文件去硬编码（删除 dotenv import + 删密码 typo 字面值 + page.goto 改相对路径）；6 条 read-only smoke 用例 @prod-safe 标签（USER×2 + ROOM×2 + RANKING×2）；12 条 specHardening TDD 验收用例（typo/localhost/dotenv/baseURL/fuzzy 拼写等 grep 反向断言）
- **AdminServer 多 profile 配置体系**: [T-10020 TDS](../tds/adminServer/T-10020.md) + [`app/adminServer/config/`](../../app/adminServer/config/) —— 与 AppServer 对称、ADMIN_PROFILE 白名单、fail-fast 错误契约、5 档 .toml 文件（default/dev/test/staging/prod）、D-A1：dev REDIS_URL 缺失 → NoopEventPublisher（0 回归）
- **Web 多 profile 环境配置体系**: [T-20020 TDS](../tds/web/T-20020.md) + [doc/arch/web/config.md](../arch/web/config.md) —— Vite mode 加载链、五档 `.env.{mode}` 文件、启动期 fail-fast 校验 `[CONFIG ERROR]` 前缀、`VITE_ADMIN_API_BASE_URL` 字段冻结、apiClient 删默认值、vitest setup.ts stub、517/517 tests passed、production bundle 0 dev URL 泄露
- **Android 多环境 productFlavors 体系**: [T-30050 TDS](../tds/android/T-30050.md) + [doc/arch/android/build.md](../arch/android/build.md) —— productFlavors {local/staging/prod} 维度、applicationIdSuffix 三档后缀、BuildConfig 三域名注入、NetworkSecurityConfig 双锁机制（manifestPlaceholder 编译期 + xml 运行时）、flavor-specific test sourceset 隔离、M2 多环境对称完整闭合（与 T-00040/T-10020/T-20020 同批完成）
