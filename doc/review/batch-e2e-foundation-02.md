# 全局代码审查报告: 模块9 E2E 测试基建 增量批次（T-0000M 双服务共库 Migration 表隔离）
> **当前状态机**：负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]

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

> 审查范围：`cf330cf..HEAD`（核心 8 个 T-0000M commit + 文档/索引同步）。已读：T-0000M TDS、ADR-0001、`directory_and_ddd.md` §3.2、`app/shared/src/migrate/mod.rs` 全文、两端 main.rs L40~75、`migration_isolation_test.rs`、`tests/common/mod.rs`、`scripts/dev/init-db.sh`、`scripts/dev/e2e-up.sh`，并跑通 `cargo test -p voice-room-shared --lib migrate::`（7/7 PASS）。
>
> 关切 ①（helper 工程正确性）：核心实现（白名单校验 / FNV-1a advisory lock / checksum & missing 检测 / 事务边界 / 表名注入闭合）与 R1 单 Task 评审一致，无新增缺陷。
> 关切 ③（脚本与文档闭环）：`init-db.sh` GRANT 幂等、`e2e-up.sh` inline workaround 已撤（`grep "GRANT CREATE" scripts/dev/e2e-up.sh` 0 命中）、AdminServer 子进程作用域 `DATABASE_URL=$ADMIN_DATABASE_URL cargo run` 不污染父 shell，全部通过。
> 关切 ②、关切 ④：发现下列阻断性问题。

- [ ] **缺陷 1**：[级别 **P1**] **本批次新立的架构规约与既有测试代码自相矛盾，14 处遗留 `sqlx::migrate!("./migrations").run(&pool)` 直连默认表违反规约**
  - **文件与行号**：
    - `app/server/tests/wallet_api_test.rs:142,187,233,339,448,586`（6 处）
    - `app/server/tests/ranking_test.rs:115,178,225,281,465`（5 处）
    - `app/server/tests/gift_list_test.rs:601,639`（2 处）
    - `app/server/tests/governance_real_repos_test.rs:33`（1 处）
  - **问题说明**：
    1. 本批次新落地 `doc/architecture/directory_and_ddd.md` §3.2.3 强制规约：**「集成测试统一走 `app/server/tests/common/mod.rs::run_migrations()`」**、**「不得直接调用 `sqlx::migrate!(...).run(&pool)`，会回退默认 `_sqlx_migrations`，破坏隔离」**。
    2. 实测 `grep -rn "sqlx::migrate" app/server/tests/` 仍有 14 处直连默认 `_sqlx_migrations` 的字面量调用，与规约同 PR 自相矛盾。
    3. 后果：dev/CI 库会同时存在 `_sqlx_migrations`（旧）+ `_sqlx_app_migrations`（新）两套登记表，长期语义不一致；新成员复制粘贴这 4 个测试文件即绕开规约，规约形同虚设。
    4. 本 Task TDS §2.3 仅声明 `wallet_schema_test`/`send_gift_test` 范围，但既然规约已升级到「全部集成测试统一入口」，遗留 14 处必须在本批次收敛——否则规约落地不完整。
  - **修复建议**：
    1. 将 4 个测试文件 14 处调用全部替换为 `mod common; common::run_migrations(&pool).await?;`；
    2. 收敛后 `grep -rn "sqlx::migrate!(\"./migrations\").run" app/server/tests/` 应仅剩 `tests/common/mod.rs` 与 `tests/migration_isolation_test.rs`（后者用例本身需要 raw migrator 调用）；
    3. 跑 `cargo test -p voice-room-server` 验证 0 回归。
  - **TDD 修复记录**：commit `1922369`（test(server): batch-02 P1.1 收敛 14 处 sqlx::migrate! 调用到 common helper）。`wallet_api_test.rs` 6 处 / `ranking_test.rs` 5 处 / `gift_list_test.rs` 2 处 / `governance_real_repos_test.rs` 1 处全部改走 `mod common; common::run_migrations(&pool).await`。验证：`grep -rn 'sqlx::migrate!("./migrations").run' app/server/tests/` 仅剩 `tests/common/mod.rs`（`migration_isolation_test.rs` 用 raw migrator 是用例本身需要，不在收敛范围）。

- [ ] **缺陷 2**：[级别 **P1**] **T-0000M DoD 第 1 条「`npm run e2e:up` 一遍过 5 端绿」未实际达成，且 /health 缺失与 ranking r08 perf flake 无显式 Task 接力，闭环存在失实风险**
  - **文件与行号**：
    - `scripts/dev/e2e-up.sh:67-68` 等待 `http-get://127.0.0.1:3000/health` 与 `http-get://127.0.0.1:3001/health`
    - `app/server/src/bootstrap/mod.rs:335` 仅注册 `/ping`（grep `app/{server,adminServer}/src` 0 命中 `/health` 路由）
    - `doc/tds/infra/T-0000M.md:192`（TDS §4 已自承：`wait-on http-get://...:3000/health` 与 `npm run preflight` 因 `/health` 缺失而失败）
    - `doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md:47`（T-0000M 行 DoD #1 仍写「`docker compose down -v && npm run e2e:up` 一遍过 5 端绿」）
    - `doc/product/index.md:48` / `doc/tasks/index.md:15`（v3.13 / v2.51 已宣告模块 9「13/13 ✅ 全闭环」）
  - **问题说明**：
    1. T-0000M 验收标准 #1 与 TDS §3.2 E-1 显式要求 5 端 wait-on 全绿；实际 `e2e-up.sh` `wait-on /health` 必超时返回非 0（`/health` 路由根本不存在），DoD #1 在客观事实层未满足。
    2. TDS §4「关键决策与坑点」自承 E-1/E-2 未通过，但只口头记入「残余风险与后续 TODO」——`grep -n "/health\|r08" doc/tasks/index.md doc/tasks/模块9*.md` 0 命中，**未生成新 Task ID（如 T-0000N）也未记入任何 backlog 文件**。
    3. `ranking_test::r08_response_time_under_100ms` perf flake 同样仅在 TDS §4 提及，无显式 follow-up，模块「13/13 ✅」标记后这两项随时可能被忘却。
    4. 这违背 batch-01 已闭环的「DoD 验证一致性」承诺：模块 9 对外宣告 ✅ 但首要 DoD 客观不绿，是**模块对外承诺失实**。
  - **修复建议**（二选一即可解锁本批次）：
    - **方案 A（推荐）**：新建 follow-up Task `T-0000N`（AppServer / AdminServer 暴露 `/health` 端点）写入 `doc/tasks/模块9-*.md` 表格，状态 🔴 待开发；同时在 T-0000M TDS §3.1 / §3.2 把 E-1/E-2 标注为「依赖 T-0000N，本 Task 不阻塞」并把 T-0000M DoD #1 改为「双服务进程冷启动均完成 migrate 且 `_sqlx_app_migrations`=9、`_sqlx_admin_migrations`=4」（这是 T-0000M 真正的承诺面）。
    - **方案 B**：在本批次内补 `/health` 路由（4~6 行 axum handler），保持 DoD #1 字面承诺。
    - **同时**：对 `ranking_test::r08` perf flake 在 backlog 立显式 Task 或 known-issue 表，注明触发条件与跳过策略。
  - **TDD 修复记录**：commit `ba00221` + `4295835`（docs: batch-02 P1.2 立 T-0000N/O follow-up + T-0000M DoD #1 修正 + product/tasks v3.14/v2.52）。采用方案 A：(1) `doc/tasks/模块9-E2E测试基建.md` 表格新增 T-0000N（AppServer/AdminServer 暴露 `/health` 端点，🔴 待开发，依赖 T-0000H/T-0000M）+ T-0000O（ranking_test::r08 perf flake known-issue，🔴 待开发）；(2) T-0000M DoD #1 措辞由「`docker compose down -v && npm run e2e:up` 一遍过 5 端绿」改为「双服务进程冷启动均完成 migrate 且 `_sqlx_app_migrations`=9 / `_sqlx_admin_migrations`=4（5 端 wait-on 全绿依赖 T-0000N）」；(3) 模块 9 完成进度由「13/13 ✅」回写为「13/13 ✅ + 2 follow-up（T-0000N/O）」；(4) `doc/tds/infra/T-0000M.md` §3.2 E-1/E-2 标注「依赖 T-0000N，本 Task 不阻塞」+ §4 关键决策与坑点指向 T-0000N/O + §4.1 新增「Round 1 修复说明」章节列出全部 6 条缺陷修复要点；(5) `doc/tasks/index.md` v2.51 → v2.52、`doc/product/index.md` v3.13 → v3.14。严守红线：未触动 batch-01 已闭环 12 个 Task 状态行；未修改 T-0000M 的 Review/QA/Overall Gate 三列。

- [ ] **缺陷 3**：[级别 **P2**] **helper 未透传 `Migration::no_tx`，未来引入 `CREATE INDEX CONCURRENTLY` 类 DDL 时会与 sqlx 原生行为分叉**
  - **文件与行号**：`app/shared/src/migrate/mod.rs:197-229`（`run_inner` step 5）
  - **问题说明**：当前 13 条迁移文件均不带 `-- no-transaction`，无运行时 bug；但 `run_inner` 对所有迁移无差别 `conn.begin()`，未读取 `Migration::no_tx`。一旦未来加 `CREATE INDEX CONCURRENTLY` 等不能在事务内执行的 DDL，PG 会抛 `25001 ACTIVE_SQL_TRANSACTION` 而 sqlx 原生 `Migrate::run` 会自动走 no-tx 分支，行为分叉且报错隐晦。
  - **修复建议**：在循环内分支 `if m.no_tx { sqlx::raw_sql(m.sql).execute(&mut *conn).await?; INSERT 也用 conn 直连 } else { 走现有 tx 路径 }`；同时给 `run_migrations_with_table` 加一条 dispatch 单元测试（构造 mock no_tx Migration），或退而求其次加一条 `if m.no_tx { return Err(...) }` fail-fast 防误用。
  - **TDD 修复记录**：commit `39c6dd7`（feat(infra): batch-02 P2.3 helper 透传 Migration::no_tx 分支）。`app/shared/src/migrate/mod.rs` `run_inner` step 5 在循环内增加 `if m.no_tx { ... } else { ... }` 分支：no_tx 路径下 DDL 与登记表 INSERT 都直接走 `&mut *conn`（不开启事务），与 sqlx 0.8 原生 `Migrate::run` 行为对齐，避免未来 `CREATE INDEX CONCURRENTLY` DDL 时报 `25001 ACTIVE_SQL_TRANSACTION`。新增单元测试 `tests::no_tx_dispatch_executes_without_transaction`：构造 `Migration { no_tx: true, sql: "SELECT 1" }` 跑通 dispatch，断言无 panic 且登记表 +1 行（DATABASE_URL 未设置则 SKIP）。`cargo test -p voice-room-shared --lib migrate::` → **8/8 PASS**（原 7 + 新增 1）。

- [ ] **缺陷 4**：[级别 **P2**] **`migration_isolation_test.rs` 顶部注释关于 `app_server_user` 可跑此测试不准确**
  - **文件与行号**：`app/server/tests/migration_isolation_test.rs:12-14`
  - **问题说明**：注释称「受限账号 `app_server_user` 在 dev 环境下有 `CREATE ON SCHEMA public` 权限，亦可执行」。但 `init-db.sh` 仅 `GRANT CREATE ON SCHEMA public`（建表权限），**未** `GRANT CREATE ON DATABASE voiceroom`（建 schema 权限）。`create_isolated_schema` 对 `app_server_user` 跑 `CREATE SCHEMA t0m_<uuid>` 必报 `permission denied`，注释会误导新成员。
  - **修复建议**：把注释改为「**仅 superuser DATABASE_URL（如 `postgres://postgres:...`）可跑**；受限账号会在 `CREATE SCHEMA` 步报权限不足」。或在 init-db.sh 追加 `GRANT CREATE ON DATABASE voiceroom TO app_server_user`（dev-only）使注释成立。
  - **TDD 修复记录**：commit `29edae7`（test(server): batch-02 P2 收尾，包含 P2.4）。`app/server/tests/migration_isolation_test.rs:12-15` 顶部注释改为：「**仅 superuser DATABASE_URL（如 `postgres://postgres:...`）可跑**；受限账号 `app_server_user` 仅有 `GRANT CREATE ON SCHEMA public`（建表权限），**无** `GRANT CREATE ON DATABASE voiceroom`（建 schema 权限），`CREATE SCHEMA t0m_<uuid>` 会报 `permission denied`。」

- [ ] **缺陷 5**：[级别 **P2**] **测试 schema 缺失 RAII guard，panic 路径 schema 残留**
  - **文件与行号**：`app/server/tests/migration_isolation_test.rs:124,159,196,245,309`
  - **问题说明**：`drop_schema` 仅在断言全绿尾部调用；任一 `assert!` 或 `.expect()` panic 时 `t0m_<uuid16>` schema 永久残留。反复跑红测试后 PG 中累积大量孤儿 schema，影响 dev DB 健康度。
  - **修复建议**：用 `scopeguard::defer!` 或 `Drop` impl 包装 schema 名，确保 panic 路径同样 `DROP SCHEMA ... CASCADE`；或在每个用例开头先 `DROP SCHEMA IF EXISTS` 兜底。
  - **TDD 修复记录**：commit `29edae7`（test(server): batch-02 P2 收尾，包含 P2.5）。新增 `SchemaGuard { pool, schema }` 结构，`Drop::drop` 中另起线程构建一次性 tokio 运行时执行 `DROP SCHEMA "<schema>" CASCADE` 并 `join()` 等待完成；与外层 `#[tokio::test]` 运行时解耦（单线程或多线程 flavor 均可用）。U-1/U-2/U-3/N-1/N-2/N-3 全部改用 `let _guard = SchemaGuard::new(setup_pool.clone(), schema.clone());`，移除尾部 `drop_schema(...)` 显式调用；panic 路径也保证 schema 清理。纯 std 实现（不引 `scopeguard` 依赖）。

- [ ] **缺陷 6**：[级别 **P2**] **N-2（权限缺失显式失败）仅在 TDS §4 手工实测说明，未沉淀为自动化测试**
  - **文件与行号**：`app/server/tests/migration_isolation_test.rs`（U-1/U-2/U-3/N-1/N-3 五例，缺 N-2）；TDS §3.3 列了 N-2 但未实现
  - **问题说明**：TDS §3.3 N-2 要求「临时 REVOKE CREATE 后启动应在错误消息中包含 `_sqlx_app_migrations` 表名」，TDD 仅在 TDS §4「关键决策与坑点」手工 `REVOKE CREATE` 实测，未化为可回归的 #[test]。后续若错误消息格式回退（例如 `MigrateTableError::sqlx` 不再含表名），无自动化护栏。
  - **修复建议**：补一条 N-2 集成用例：连接 superuser 起 schema → 在该 schema 内 `REVOKE CREATE ON SCHEMA <schema> FROM CURRENT_USER` → 用受限连接跑 helper → 断言 `err.to_string().contains("_sqlx_app_migrations")`。或显式在 TDS §3.3 把 N-2 标记为「手工验收，不入自动化套件」并接受。
  - **TDD 修复记录**：commit `29edae7`（test(server): batch-02 P2 收尾，包含 P2.6）。新增 `n2_revoke_create_emits_table_name_in_error`：superuser 建 schema → `GRANT USAGE ON SCHEMA <s> TO app_server_user`（仅 USAGE，不 GRANT CREATE）→ 用 `app_server_user` 受限连接（带 `search_path=<s>`）跑 `run_migrations_with_table` → 断言 `err.to_string().contains("_sqlx_app_migrations")`。沉淀 TDS §3.3 N-2 手工实测为可回归用例。`cargo test -p voice-room-server --test migration_isolation_test` → **6/6 PASS**（原 5 + N-2，no_tx 单元测试落在 shared crate 不重复落此处）。

---

**@GlobalReview 总评**：❌ **不通过**。

- 关切 ①、③ 通过（helper 工程正确性、脚本与文档闭环 OK；R1 已细审，本轮复核无新增问题）。
- 关切 ②、④ 不通过：(a) 14 处遗留默认表调用与本批次新立的架构规约 `directory_and_ddd.md` §3.2.3 自相矛盾，是**长期可维护性**红线（缺陷 1，P1）；(b) T-0000M DoD #1「5 端 wait-on 全绿」客观未达成、`/health` 缺失与 ranking r08 perf flake 无显式 follow-up Task 接力，模块 9「13/13 ✅」标记存在**对外承诺失实**风险（缺陷 2，P1）。
- 单 Task R1 提的 4 条 follow-up，本批次判定：
  - **升级为阻断（P1）**：第 3 条「其余 5 个 server 集成测试遗留 `MIGRATOR.run`」→ 本审查缺陷 1（理由：与同 PR 落地的架构规约直接冲突）；
  - **保持 P2**：第 1 条 `no_tx` → 缺陷 3；第 2 条 `app_server_user` 注释 → 缺陷 4；第 4 条 RAII guard → 缺陷 5。
- 额外新增：缺陷 2（DoD 失实闭环）、缺陷 6（N-2 自动化缺失）。

请 TDD 优先消化 P1 两条（缺陷 1、2），P2 四条可在同一轮一并处理。修复完成后将状态机改为 `负责人 [GlobalReview] | 状态 [⏳ In Review]` 触发下一轮审查。

*(请在文档头部将状态机修改为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`)*

---

### 【第 2 轮审查】
**@GlobalReview 复审意见：**

> 复审范围：`ac1a2a1..HEAD` 共 6 个 commit（`1922369` / `39c6dd7` / `29edae7` / `ba00221` / `4295835` / `c4cbb2d`）。已实跑 `cargo test -p voice-room-shared --lib migrate::`、`cargo test -p voice-room-server --test migration_isolation_test`、`cargo test -p voice-room-server` 全套（DATABASE_URL=postgres superuser + APP_SERVER_DATABASE_URL=app_server_user）。

**Round 1 缺陷核验：**

- [✅] **缺陷 1（P1，14 处遗留收敛）**：`grep -rn 'sqlx::migrate!("./migrations").run' app/server/tests/` 仅命中 `common/mod.rs:3` 文档注释；`grep -rn 'sqlx::migrate!' app/server/tests/` 剩余命中均为 `common/mod.rs:22`（helper 自身）+ `migration_isolation_test.rs` 6 处（用例本身需要 raw `Migrator` 注入异常迁移文件，合法保留）。`grep -rn "common::run_migrations" app/server/tests/` 命中 26 处，覆盖 wallet_api_test (6) / ranking_test (5) / gift_list_test (2) / governance_real_repos_test (1) / wallet_schema_test / send_gift_test。`cargo test -p voice-room-server` 全套 0 失败（`test result: ok` 行 0 fail across 8 binaries），含 `migration_isolation_test 6/6 PASS`。规约与代码一致，commit `1922369` 收敛完成。

- [✅] **缺陷 2（P1，DoD 失实闭环）**：
  - `doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md` line 48 / 49 完整新增 T-0000N（AppServer/AdminServer `/health`，依赖 T-0000H+T-0000M，🔴 待开发）+ T-0000O（ranking r08 perf flake known-issue，🔴 待开发），表格列齐全（归属/依赖/描述/DoD/工时/Review/QA/Overall Gate）。
  - line 47 T-0000M DoD #1 实测文本：「双服务进程冷启动均完成 migrate 且 `_sqlx_app_migrations`=9、`_sqlx_admin_migrations`=4（5 端 wait-on 全绿依赖 T-0000N）」，符合事实承诺。
  - `doc/tds/infra/T-0000M.md` §3.2 line 143/144 E-1/E-2 标注「**【依赖 T-0000N，本 Task 不阻塞】**」；§4.1 line 196 新增「Round 1 修复说明（2026-04-27）」章节，列出 6 条缺陷修复要点 + 回归结果（line 212-215）。
  - `doc/product/index.md` v3.14（line 3）+ changelog（line 67）；`doc/tasks/index.md` v2.52（line 3）+ changelog（line 15）回写「13/13 ✅ + 2 follow-up」。
  - 模块 9 task 文件 line 51 汇总「13/13 ✅ + 2 follow-up（T-0000N / T-0000O 待开发）」一致。
  - **唯一遗留小不一致**：line 7「**状态**：🟢 已闭环（13/13 ✅ Done…）」未补上 follow-up 字样，与 line 51 汇总在措辞口径上不齐。本 Reviewer 明确判定：**不升级为 P1 阻断**，理由——(a) 协调器红线明确禁止本 Reviewer 修改模块 9 task 文件，按文档约定该口径回流由 Coordinator 闭环时同步主表；(b) 实质内容（T-0000N/O 行 + DoD #1 + 汇总）已诚实反映现状。仅以 P2 级建议留档，见下文「新增问题」。

- [✅] **缺陷 3（P2，no_tx 透传）**：`app/shared/src/migrate/mod.rs:197-254` `run_inner` step 5 内循环已分支 `if m.no_tx { sqlx::raw_sql + INSERT 都走 &mut *conn 不开 tx } else { conn.begin() → tx → commit }`，no_tx 路径注释明确说明对齐 sqlx 0.8 原生 `Migrate::run`、失败时部分语句已提交需运维介入。新增单元测试 `tests::no_tx_dispatch_executes_without_transaction` 已落地。`cargo test -p voice-room-shared --lib migrate::` → **8/8 PASS**（`validate_*` 5 + `lock_id_*` 1 + `no_tx_dispatch_*` 1 + 1 = 8 实测）。

- [✅] **缺陷 4（P2，注释修正）**：`app/server/tests/migration_isolation_test.rs:12-15` 当前文本：「注意：本测试**仅 superuser DATABASE_URL（如 `postgres://postgres:...`）可跑**；受限账号 `app_server_user` 仅有 `GRANT CREATE ON SCHEMA public`（建表权限），**无** `GRANT CREATE ON DATABASE voiceroom`（建 schema 权限），`CREATE SCHEMA t0m_<uuid>` 会报 `permission denied`。」措辞符合 Round 1 修复要求，准确无误导。

- [✅] **缺陷 5（P2，RAII guard）**：`migration_isolation_test.rs:65-93` `SchemaGuard { pool, schema }` 结构 `Drop::drop` 中另起 `std::thread + tokio current_thread rt` 跑 `DROP SCHEMA IF EXISTS "<schema>" CASCADE` 并 `join()`，与外层 `#[tokio::test]` 运行时解耦。`grep -n "SchemaGuard::new\|drop_schema(" migration_isolation_test.rs` 显示 6 处 `SchemaGuard::new(...)`（U-1/U-2/U-3/N-1/N-2/N-3 全覆盖），无任何裸调 `drop_schema(...)`，panic 路径有 RAII 兜底。`drop_schema` 函数本身保留为 `#[allow(dead_code)]` 工具，无副作用。纯 std 实现，未引入 `scopeguard` crate，符合最小依赖原则。

- [✅] **缺陷 6（P2，N-2 自动化）**：`migration_isolation_test.rs:362` `n2_revoke_create_emits_table_name_in_error` 实现路径：superuser 起 schema → 仅 `GRANT USAGE ON SCHEMA <s> TO app_server_user`（不 GRANT CREATE）→ 用 `app_server_user` 受限连接（`search_path=<s>`）跑 `run_migrations_with_table` → 断言 `err.to_string().contains("_sqlx_app_migrations")`。SKIP 守卫双层（DATABASE_URL / APP_SERVER_DATABASE_URL 缺失或受限连接失败均 SKIP）。`cargo test -p voice-room-server --test migration_isolation_test` → **6/6 PASS**（u1/u2/u3/n1/n2/n3 全绿，5.70s）。

**新增问题**：

- [P2 / LOW] **模块 9 task 文件状态描述与汇总口径轻微不一致**：`doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md:7` 状态行仍写「🟢 已闭环（13/13 ✅ Done；T-0000M 双服务共库 Migration 表隔离 DoD 完成，2026-04-27）」未提 T-0000N / T-0000O follow-up；而 line 51 汇总写「13/13 ✅ + 2 follow-up（T-0000N / T-0000O 待开发）」。建议 Coordinator 闭环本批次时一并把 line 7 改为「🟢 已闭环（13/13 ✅ + 2 follow-up：T-0000N / T-0000O 待开发）」保持口径一致。**不阻塞本批次放行**——本 Reviewer 严守红线不修改模块 9 task 文件，留给 Coordinator 闭环。
- 未发现 Round 1 修复引入的新 bug 或新设计裂缝；6 个 commit 边界清晰、单一职责、无跨模块串味。

**TDD 修复记录填写情况**：6/6 全部填写完整 commit ID（`1922369` / `ba00221+4295835` / `39c6dd7` / `29edae7` / `29edae7` / `29edae7`），每条均有「修复路径 + 验证证据」描述，符合流程要求。

**@GlobalReview 总评**：🟢 **通过**。
- 通过条件全部满足：6 条 Round 1 缺陷全部 ✅，无残留 P1，无新增阻断；唯一遗留为 P2/LOW 文档口径小不一致，按红线不修改、留 Coordinator 闭环。
- 关切 ①（helper 工程正确性）+ 关切 ② / ④（替换覆盖度 + 残余风险可追溯性）+ 关切 ③（脚本与文档闭环）本轮全部出清。
- 实跑回归：shared `migrate::` 8/8 / server `migration_isolation_test` 6/6 / `cargo test -p voice-room-server` 全套 0 fail，无回归。

*(请在文档头部将状态机修改为：`负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]`)*

---
