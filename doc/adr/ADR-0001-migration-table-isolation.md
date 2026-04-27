# ADR-0001: 双服务共库 Migration 表隔离

- **状态**：Accepted
- **日期**：2026-04-27
- **决策者**：Plan Agent（Architect）
- **关联 Task**：[T-0000M](../tds/infra/T-0000M.md)
- **关联模块**：模块 9 E2E 测试基建（M4 里程碑）

## 1. 背景（Context）

Voice Room 当前为「模块化单体 + 双进程」架构：`voice-room-server`（C 端）与 `voice-room-admin-server`（B 端）共享同一 PostgreSQL 库 `voiceroom`，按角色（`app_server_user` / `admin_server_user`）做表级权限隔离。两进程各自维护独立 `migrations/` 目录，启动时调用 `sqlx::migrate!("./migrations").run(pool)`。

2026-04-27 在 `npm run e2e:up` 全栈冷启动联调中暴露架构级阻断：

| 服务 | 实测错误 | 根因 |
|------|----------|------|
| AppServer | `permission denied for table _sqlx_migrations` | AdminServer 抢先建表，所有者 = `admin_server_user` |
| AdminServer | `migration 5 was previously applied but is missing in the resolved migrations` | AppServer 已写入 v5..9，AdminServer 自身仅 v1..4，sqlx 视为「迁移消失」 |

`docker compose down -v` 全清后必现复发——只要双服务对同一库执行默认 `_sqlx_migrations` 表，**版本号 / checksum 必然互掐**。

## 2. 备选方案（Options）

### 方案 A：物理双库（`voiceroom_app` / `voiceroom_admin`）

- **优点**：最干净的服务间隔离，符合微服务边界理论。
- **代价（致命）**：经源码盘点，AdminServer 已存在 **13 处直接 `SELECT … FROM users / rooms / wallets / events`** 的跨域查询：
  - [`modules/user/repository.rs`](../../app/adminServer/src/modules/user/repository.rs) ×3
  - [`modules/room/repository.rs`](../../app/adminServer/src/modules/room/repository.rs) ×3
  - [`modules/wallet/repository.rs`](../../app/adminServer/src/modules/wallet/repository.rs) ×2
  - [`modules/stats/repository.rs`](../../app/adminServer/src/modules/stats/repository.rs) ×2
  - [`modules/event/query_repo.rs`](../../app/adminServer/src/modules/event/query_repo.rs) ×3
- 双库后这 13 处必须改写为 RPC 调 AppServer 或启用 `postgres_fdw`，对 MVP 节奏过度工程，且与 `doc/architecture/directory_and_ddd.md` 既定的「共享 DB、按权限隔离」方向冲突。

### 方案 B：双 migration 表（自定义 `_sqlx_app_migrations` / `_sqlx_admin_migrations`）

- **优点**：仅改两个 `main.rs` 各 ≤ 10 行；用 sqlx 官方 `Migrator::set_table_name` API；零业务改动；零数据迁移。
- **代价**：偏离 `sqlx::migrate!` 宏默认约定（`_sqlx_migrations`），需要在架构文档显式声明。

### 方案 C：启动期跳过 migrate + sqlx-cli 一次性执行

- **优点**：最贴近生产实践（运行时账号无需 schema CREATE 权限）。
- **代价**：需引入 `sqlx-cli` 依赖与 CI 步骤；本地 DX 退化（多一步手动迁移）。
- **关键认识**：C 不是 B 的替代，而是 B 的**超集**——`sqlx migrate run` 同样需要 `--migrations-table` 自定义表名来解决「同表两份历史并存」的语义问题。**B 是必经路径**。

## 3. 决策（Decision）

**采纳方案 B 作为 Phase 1 解法，方案 C 列为 Phase 2 演进项，方案 A 否决。**

### 表名约定

| 进程 | 迁移登记表 | 创建者角色 |
|------|------------|------------|
| `voice-room-server` (AppServer) | `_sqlx_app_migrations` | `app_server_user` |
| `voice-room-admin-server` (AdminServer) | `_sqlx_admin_migrations` | `admin_server_user` |

### 实施手段

两服务 `main.rs` 由

```rust
sqlx::migrate!("./migrations").run(&pool).await?;
```

改为：

```rust
voice_room_shared::migrate::run_migrations_with_table(
    &pool,
    &sqlx::migrate!("./migrations"),
    "_sqlx_app_migrations",   // AdminServer 用 _sqlx_admin_migrations
).await?;
```

#### sqlx Migrator API 选型回溯（TDD spike 实测）

TDS §2.2 列出三档路径：首选「`migrate!().clone() + set_table_name`」、降级「`Migrator::new(Path)`」、保底「手写 SQL」。TDD 阶段实测发现：

- **首选不可行**：sqlx 0.8.6 的 `Migrator` 结构体**不含** `table_name` 字段（`pub migrations / ignore_missing / locking / no_tx`），也**不提供** `set_table_name` / `with_table_name` 方法。
- **降级同样不可行**：`Migrator::new(Path)` 返回的是同一个结构体，仍走 `sqlx-postgres-0.8.6/src/migrate.rs` L119-310 的硬编码 SQL（`CREATE TABLE IF NOT EXISTS _sqlx_migrations`、`SELECT ... FROM _sqlx_migrations` ×3、`UPDATE _sqlx_migrations`、`INSERT INTO _sqlx_migrations`、`DELETE FROM _sqlx_migrations`），表名无法注入。
- **故采纳保底方案**：在 `voice-room-shared` 新增 `migrate` 模块，复用 `sqlx::migrate!()` 宏暴露的 `Migrator::iter()`（含 version / description / checksum / sql 全部信息），仅自管登记表 SQL；表名经白名单校验防注入。

未来 sqlx ≥ 0.9 一旦发布 `set_table_name` 官方 API（已在 main 分支 RFC，alpha.1 待验），即可把 helper 退化为薄封装。

同步在 [`scripts/dev/init-db.sh`](../../scripts/dev/init-db.sh) 收口 `GRANT CREATE ON SCHEMA public TO app_server_user`，撤掉 [`scripts/dev/e2e-up.sh`](../../scripts/dev/e2e-up.sh) 的 inline workaround。

## 4. 后果（Consequences）

### 正面

- `docker compose down -v && npm run e2e:up` 冷启动一次绿，解除模块 9 M4 阻断。
- 双服务迁移演进互不感知，新增迁移文件不会影响对方启动。
- 与既有「共享 DB + 角色权限隔离」架构方向一致，不引入新组件。

### 负面 / 风险

- 偏离 sqlx 宏默认约定，新成员需查阅本 ADR 或架构文档才能理解 `_sqlx_app_migrations` 表名来由。
- 既有 dev / staging 库可能残留旧 `_sqlx_migrations` 表，需在升级路径中说明（旧表保留不动，不污染新表；如需清理可 `DROP TABLE _sqlx_migrations`，仅 dev 安全）。
- `app_server_user` 在 dev 环境需保留 `CREATE ON SCHEMA public` 权限（生产环境切 C 方案后即可收回）。

### 演进路线

| 阶段 | 触发条件 | 演进动作 |
|------|----------|----------|
| **现在（B）** | T-0000M | 双 migration 表，runtime migrate-on-startup |
| **Phase 2（C）** | 上 staging / prod 时 | 引入 `sqlx-cli`，CI/部署脚本一次性 `sqlx migrate run --migrations-table …`；两服务通过 env flag `MIGRATE_ON_START=0` 跳过 runtime migrate |
| **Phase 3（A）** | 满足任一信号：①AdminServer 跨域查询 < 3 处；②C/B 端数据安全审计要求物理隔离；③多租户 SaaS 化 | 物理双库 + AdminServer 改造为 RPC / FDW |

## 5. 参考

- TDS：[T-0000M 双服务共库 Migration 表隔离](../tds/infra/T-0000M.md)
- 任务台账：[doc/tasks/index.md](../tasks/index.md) v2.50
- 模块 9：[E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md) M4 里程碑
- 受影响代码：[`app/server/src/main.rs`](../../app/server/src/main.rs#L56)、[`app/adminServer/src/main.rs`](../../app/adminServer/src/main.rs#L62)
- sqlx 文档：`sqlx::migrate::Migrator::set_table_name`
