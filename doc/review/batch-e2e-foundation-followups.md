# 全局代码审查报告：模块 9 E2E 测试基建 follow-up 增量批次（T-0000N + T-0000O）

> **当前状态机**：负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]

---

## 0. 流转规则

- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由 [GlobalReview] 进行全局代码审查。
- [GlobalReview] 审查通过 → 修改负责人 [-] 状态 [✅ Passed]。
- [GlobalReview] 审查未通过 → 修改负责人 [TDD] 状态 [❌ Failed]，并将审查意见追加到文档下方。
- [TDD] 修复并自测后 → 状态改为负责人 [GlobalReview] 状态 [⏳ In Review]，触发下一轮复审。

---

## 1. 审查上下文

- **审查范围**：模块 9 主批次（A: 12 任务 / B: T-0000M）已合并闭环至 [模块9-E2E测试基建.md](./模块9-E2E测试基建.md)。本 follow-up 批次仅审查批次 B 闭环后追加的 2 个 follow-up 任务的全量 TDD 交付物（代码 + 文档 + 测试），**不重复审查**已 Passed 的 13 个 Task。
- **包含任务模块**：[模块 9: E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)
- **包含任务**：T-0000N、T-0000O（共 2 个 follow-up）
- **关联 TDS**：
  - [T-0000N](../tds/infra/T-0000N.md)（AppServer / AdminServer 暴露统一 `/health` 端点）
  - [T-0000O](../tds/infra/T-0000O.md)（`ranking_test::r08` perf flake known-issue 收口）
- **代码 diff 范围**：`d922e72..HEAD`（自批次 B 合并 commit `05eaa47` 之后）
  - `d1454e0` feat(infra): T-0000N AppServer/AdminServer 暴露统一 /health 端点
  - `c2181c0` review(infra): T-0000N Round 1 🟢通过（TDS-level）
  - `743f14e` docs(arch): T-0000N DoD 同步 /health 端点说明 + tasks v2.53 changelog
  - `2c07b7b` chore(tasks): T-0000N 研发闭环（Dod ✅ Done）
  - `b793252` test(server): T-0000O r08 perf flake 收口（#[ignore] + known-issues 登记）
  - `ae20b9f` review(infra): T-0000O Round 1 🟢通过（TDS-level）
  - `3e28148` docs(tasks): T-0000O DoD 同步 v2.54 changelog
  - `4b027e4` chore(tasks): T-0000O 研发闭环（Dod ✅ Done）
- **开始时间**：2026-04-28

---

## 2. 审查关切（来自协调者）

主批次已闭环模块 9 的整体架构与基建可用性。本 follow-up 批次只追问与这两个收口任务强相关的**架构级 + 工程级**问题，避免重复劳动：

### 关切 ①：T-0000N `/health` 端点的工程正确性

- AppServer / AdminServer 两端 `/health` 路由实现是否一致：响应体 schema（`{status:"ok", service, version}`）、HTTP 200、纯静态、不依赖外部资源（DB / Redis / 下游服务）；
- 是否在路由层注册到**无鉴权**位置（不被 JWT / Admin Auth 中间件拦截）；
- `npm run e2e:up` 中 `wait-on http-get://...:3000/health` 与 `http-get://...:3001/health` 是否真生效（可冷启实测；如有未生效情况属本批次缺陷）；
- `scripts/dev/preflight.sh` 是否真切到 `/health` 而非保留旧的 `/ping`；
- 文档同步：`doc/arch/server/index.md`、`doc/arch/adminServer/index.md` 是否记录 `/health` 端点契约。

### 关切 ②：T-0000O perf flake 收口的工程正确性

- `#[ignore = "perf flake; tracked by T-0000O"]` 注解：Rust 注解语法是否合法、消息字符串是否符合 cargo test 行为（`cargo test` 默认 skip + 输出 `ignored, "perf flake; tracked by T-0000O"`，`-- --ignored` 可单跑）；
- `doc/tests/known-issues.md` 5 必填字段（现象 / 触发条件 / 规避策略 / 手动跑命令 / 长期方向）是否齐全且可机读；
- `doc/tests/E2E_RUNBOOK.md` 是否新增链向 `known-issues.md` 的故障排查行；
- 默认 `cargo test -p voice-room-server` 是否 0 fail 且不含 r08 输出（实测）；
- `cargo test -p voice-room-server -- --ignored --test-threads=1 ranking_test::r08` 单跑稳定通过（可选实测）；
- 是否有自动化护栏（lint / CI / TDS 互链），防止后续误删 `#[ignore]` 标签或 `tracked by T-0000O` 反向引用。

### 关切 ③：批次合并风险

- 这两个 follow-up 是否触动了主批次已闭环代码（不应有非本任务路径外的代码改动）；
- 主表 `Review Gate` 占位与本批次文件路径是否一致；
- TDS-level Round 1 已 🟢，但**全局架构级审查**视角下是否存在未覆盖的横切风险（例如 `/health` 与未来 K8s readiness/liveness 的契约对齐、known-issues.md 是否纳入 doc index）。

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见：**

**审查范围**：`d922e72..HEAD` 共 8 个 commits；实质代码改动仅 `d1454e0`（T-0000N）+ `b793252`（T-0000O），其余均为 review/DoD/changelog 文档。已逐一比对：
- 改动文件全集 = `app/server/src/bootstrap/mod.rs` / `app/server/tests/health_endpoint_test.rs` / `app/server/tests/ranking_test.rs` / `app/adminServer/src/bootstrap/mod.rs` / `app/adminServer/tests/health_endpoint_test.rs` / `doc/tds/infra/T-0000N.md` / `doc/tds/infra/T-0000O.md` / `doc/tests/known-issues.md` / `doc/tests/E2E_RUNBOOK.md` / `doc/arch/server/index.md` / `doc/arch/adminServer/index.md` / `doc/tasks/index.md` / `doc/tasks/模块9-…md` / `doc/review/batch-e2e-foundation-followups.md`。
- **未触动**主批次 13 任务的任何业务路径（路由表、模块边界、E2E 配置、scripts 主体均仅在 wait-on/preflight 行做了 `/ping`→`/health` 的最小切换）。批次合并风险（关切 ③ 第 1 项）✅ 通过。

---

**关切 ① — T-0000N `/health` 端点工程正确性**

- **Schema 一致性**（U-1/U-2）✅：AppServer 与 AdminServer 均挂载 `Router::new().route("/health", get(health))`，handler 共用同一 `HealthResponse { status, service, version }` 形状；`status="ok"` 静态、`service` 分别为 `"app-server"` / `"admin-server"`、`version` 来自 `env!("CARGO_PKG_VERSION")` 编译期注入；零 AppState 读取、零 DB/Redis/下游探测 — 与 TDS 契约完全一致。
- **HTTP 语义**（N-1）✅：仅注册 `get(health)`；`POST /health` 由 axum 自动产出 `405 Method Not Allowed`，集成测试 `post_health_returns_405_method_not_allowed` 已覆盖。
- **免鉴权位置**（U-3/U-4）✅：
  - AppServer：`build_app` 顶层 `Router` 不加 auth layer，鉴权由 `auth_routes()` 等业务子模块内部按需挂载，`/health` 与 `/ping` 同层裸挂、必然免鉴权。
  - AdminServer：`audit_middleware` 通过 `match_audit_route` 仅命中 `POST /api/v1/admin/users/{id}/ban|unban` 与 `DELETE /api/v1/admin/rooms/{id}` 白名单（见 `app/adminServer/src/common/middleware/audit.rs:64-79`），`/health` 不在白名单内，逻辑上直接旁路；`request_context_middleware` 仅注入 trace context，不做鉴权决策。结论：`/health` 在 AdminServer 真实路由链路下也免 admin JWT。
- **wait-on 与 preflight 真切换**✅：`scripts/dev/e2e-up.sh:57-58` 已切到 `http-get://127.0.0.1:3000/health` + `:3001/health`；`scripts/dev/preflight.sh:152,157` 已切到 `${...}/health`，旧 `/ping` 在两脚本中均已彻底移除（`grep` 确认无残留）。
- **文档同步**✅：`doc/arch/server/index.md:36` 与 `doc/arch/adminServer/index.md:254` 均新增 `/health` 契约描述（200 OK / 三字段 schema / 零鉴权零依赖 / 用途）。

**关切 ② — T-0000O perf flake 收口工程正确性**

- **#[ignore] 落点与语法**（U-1/U-3）✅：`app/server/tests/ranking_test.rs:458` 在 `#[tokio::test]` 之后追加 `#[ignore = "perf flake; tracked by T-0000O"]`，cargo test 默认输出 `ignored, "perf flake; tracked by T-0000O"`，`-- --ignored` 显式启用；属性宏顺序合法、字符串字面量合法。测试体（100ms 阈值 / 测试逻辑）零改动，未来去 ignore 时即恢复原行为（R-3 ✅）。
- **known-issues.md 5 必填字段**（D-1）✅：`doc/tests/known-issues.md` 新建并以 `<a id="r08"></a>` 显式锚点登记，5 字段（现象 / 触发条件 / 临时规避 / 手动跑命令 / 长期方向）完整且可机读，并附上「跟踪 Task: T-0000O」反向引用 — 反链双向闭环。
- **RUNBOOK 链向**（D-2）✅：`doc/tests/E2E_RUNBOOK.md:221-223` 新增 §7 Q9，相对路径 `./known-issues.md#r08` 与新建文件锚点匹配可达。
- **回归实测**：从 commit message 与 TDS 实现结果记录看，TDD 已在本地确认 `cargo test -p voice-room-server` 默认 `10 passed; 0 failed; 1 ignored`。本审查未在 reviewer 沙箱中重跑（无 docker postgres/redis），但静态路径上 `#[ignore]` 注解必然生效，cargo 行为合约稳定，可信。
- **护栏**：当前依赖 commit message + TDS 双向引用 + RUNBOOK 链向 + Task 表 tracked-by 字段共 4 处文本反链作为「软护栏」防止误删，未引入 lint/CI hard-gate。属可接受现状（见下方 P3-2）。

**关切 ③ — 全局架构横切**

- **K8s readiness/liveness 契约对齐**：当前 `/health` 设计语义 = 「进程存活并能响应 HTTP」（liveness），零依赖故不区分 readiness。未来若引入 K8s 编排且需要 readiness（DB/Redis 连通）时需新增 `/readyz` 与之分离 — 不属本批次义务，记入未来工作即可。
- **doc index 收纳**：`doc/tests/known-issues.md` 已被 `E2E_RUNBOOK.md` 直链；但 `doc/tests/index.md` 文件清单中未追加新条目（见 P3-1）。

---

**本轮发现汇总**

🟢 **未发现任何 P0 / P1 / P2 级别缺陷。** 代码、测试、文档、脚本切换均严格符合两份 TDS 与项目架构规范。

仅记录 2 条 **P3（建议项，不阻塞放行）**：

- [ ] **P3-1**：`doc/tests/known-issues.md` 未登记入 `doc/tests/index.md` 的「文件清单」表。
  - **文件**：`doc/tests/index.md`（与 RUNBOOK 行同表）
  - **说明**：新建文档已被 RUNBOOK 直链，可达性 OK；但 `doc/tests/index.md` 是该目录的入口索引，新增 reference doc 应顺位入册以提升可发现性。
  - **建议**：在表格中追加 `| [known-issues.md](./known-issues.md) | 测试套件已知 flake / 环境性问题登记册（5 字段模板） | T-0000O |`。
  - **TDD 修复记录**：[非阻塞，可在下一次 doc 维护批次顺手补]

- [ ] **P3-2**：`#[ignore = "perf flake; tracked by T-0000O"]` 标签缺少自动化反向校验。
  - **文件**：`app/server/tests/ranking_test.rs:457-458` / CI 流水线
  - **说明**：当前防误删机制仅靠多处文本反链（commit / TDS / RUNBOOK / Task 表），无 grep-gate 或 CI lint 在「ignore 被删除」或「tracked-by 字符串变更」时主动报警。考虑到 r08 长期方向是迁出独立 perf 套件后整体重构，本约束的紧迫性不高。
  - **建议**：可选地在 `.github/workflows/*` 或 pre-commit hook 中加 `grep -q "tracked by T-0000O" app/server/tests/ranking_test.rs` 检查；或在 T-0000O Phase 2 立项时一并交付。
  - **TDD 修复记录**：[非阻塞；建议归并入未来 Phase 2 perf 套件迁移工单]

**本轮结论**：✅ **审查通过**：T-0000N 与 T-0000O 在代码实现、测试覆盖、脚本切换、文档同步、批次隔离等所有关切维度均达标；仅余 2 条 P3 文档/护栏建议，不阻塞放行。
*(已在文档头部将状态机修改为：`负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]`)*

---
