# 全局代码审查报告：模块 9 E2E 测试基建 follow-up 增量批次（T-0000N + T-0000O）

> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

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

_(等待 global-code-reviewer 填写)_

---
