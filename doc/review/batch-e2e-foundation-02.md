# 全局代码审查报告: 模块9 E2E 测试基建 增量批次（T-0000M 双服务共库 Migration 表隔离）
> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

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

- **审查范围**：模块 9 E2E 测试基建增量批次。`batch-e2e-foundation-01.md` 闭环后，模块 9 在联调阶段又暴露一个架构级阻断（双服务共库共享 `_sqlx_migrations` 表互掐），新增 T-0000M 收口；本批次**仅审查 T-0000M 全量 TDD 交付物**，不重复审 batch-01 已 Passed 任务。
- **包含任务模块**：[模块 9: E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)
- **包含任务**：T-0000M（共 1 个）
- **关联 TDS**：
  - [T-0000M](../tds/infra/T-0000M.md)（双服务共库 Migration 表隔离 TDS，含 §4 实现结果与 §5 单 Task Reviewer Round 1 🟢 通过结论）
  - [ADR-0001](../adr/ADR-0001-migration-table-isolation.md)（方案 A/B/C 决策记录与 Phase 2 切 C 触发条件）
- **代码 diff 范围**：`cf330cf..HEAD`（含 8 个 commit：联调修复 + helper 实现 + 集成测试 + 脚本收口 + ADR-0001 + 任务索引同步 + Round 1 reviewer 意见 + DoD 文档同步）
- **开始时间**：2026-04-27

---

## 2. 审查关切（来自协调者）

batch-01 已闭环模块 9 的「能否快速 E2E / 是否一键部署 / 工程素质」三大关切。本批次只追问与 T-0000M 强相关的**架构级**问题，避免重复劳动：

### 关切 ①：双服务共库 Migration 隔离的工程正确性
- `voice_room_shared::migrate::run_migrations_with_table` 自管 SQL 是否完备复刻了 sqlx 0.8 原生 `Migrate::run` 的语义边界（version 单调、checksum 校验、缺失迁移检测、advisory lock 互斥、事务/no_tx 分支）？
- `validate_table_name` 白名单 `^[A-Za-z_][A-Za-z0-9_]{0,62}$` 是否真正阻断了所有 SQL 拼接位置的注入面？
- FNV-1a(table_name) 派生的 advisory lock_id 与 sqlx 默认按 database name 派生的 lock 是否完全错开、不会在双服务并发启动时互锁？

### 关切 ②：替换覆盖度与回归风险
- 两个 `main.rs` 调用点是否一致（错误传播、表名常量是否冻结）？是否还有遗留的 `MIGRATOR.run(&pool)` 字面量调用？
- `migration_isolation_test`（U-1/U-2/U-3/N-1/N-3）+ `wallet_schema_test`（9 处替换）+ `send_gift_test`（13 处替换）是否充分？测试间 schema 隔离是否健壮（panic 路径残留 schema 风险）？
- 单 Task Reviewer 在 §5 Round 1 提出的 4 条 follow-up 建议（`no_tx` 透传缺失、test 注释关于 `app_server_user` 不准确、其余 5 个测试文件遗留 `MIGRATOR.run`、测试 schema RAII guard），是否需要**升级为本批次阻断项**或继续按 follow-up 处理？

### 关切 ③：脚本与文档闭环
- `scripts/dev/init-db.sh` 的 `GRANT CREATE ON SCHEMA public TO app_server_user` 是否幂等？`scripts/dev/e2e-up.sh` 的 inline workaround 是否完全撤回（`grep "GRANT CREATE" e2e-up.sh` 应为 0 命中）？
- AdminServer 子进程作用域的 `DATABASE_URL="$ADMIN_DATABASE_URL"` 覆盖是否正确，不会污染父 shell 或与 AppServer 互串？
- `doc/arch/server/index.md`、`doc/arch/adminServer/index.md`、`doc/architecture/index.md`、`doc/adr/ADR-0001-migration-table-isolation.md` 与 `doc/tasks/index.md` v2.51 changelog 是否一致地反映了「保底方案落地 + Phase 2 切 C 触发条件」？

### 关切 ④：残余风险的可追溯性
- TDS §4 已记录 E-1/E-2 因 `/health` 端点缺失（T-0000H 起的预存在缺陷）受限、`ranking_test::r08_response_time_under_100ms` 偶发 perf flake。这两项是否在文档中有明确去向（独立 Task 或 follow-up backlog），不被 T-0000M DoD 隐式吞掉？

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**
- 待 GlobalReview 智能体填写

---
