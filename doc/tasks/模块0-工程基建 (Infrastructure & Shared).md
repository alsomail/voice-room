# 模块 0: 工程基建 (Infrastructure & Shared)

> 返回 [任务总索引](./index.md)

## Phase 0: MVP 基础设施 (预计 6-8 周)


## 模块 0: 工程基建 (Infrastructure & Shared)

> **说明**：此模块是所有端的前置依赖，必须最先完成。

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-0000A** | 基建 | Infra | Docker Compose 开发环境 [TDS](../tds/infra/T-0000A.md) | 无 | 编写 docker-compose.yml，包含 PostgreSQL + Redis | 1. `docker-compose up` 一键启动<br>2. PG 端口 5432, Redis 端口 6379<br>3. 数据挂载本地目录，重启不丢 | 3 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-INFRA/Report.md) | ⏳ Pending |
| **T-0000B** | 基建 | Shared | 共享 crate (shared/) [TDS](../tds/infra/T-0000B.md) | 无 | 创建 Rust workspace 共享 crate，包含数据库 models、公共错误码、JWT 工具 | 1. App Server 和 Admin Server 均可引用<br>2. 包含 UserModel, RoomModel 等结构体<br>3. 包含 JWT encode/decode 函数<br>4. 包含 bcrypt 密码工具 | 5 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-INFRA/Report.md) | ⏳ Pending |
| **T-0000C** | 基建 | Infra | 数据库权限隔离 [TDS](../tds/infra/T-0000C.md) | T-0000A | 创建两个 PG Role: app_server_user (受限写) 和 admin_server_user (全权) | 1. app_server_user 只能 CRUD 指定表<br>2. admin_server_user 拥有全部权限<br>3. 提供初始化 SQL 脚本 | 2 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-INFRA/Report.md) | ⏳ Pending |
| **T-0000D** | 基建 | Infra | CI 基础流水线 [TDS](../tds/infra/T-0000D.md) | T-0000B | GitHub Actions: lint + test + build | 1. PR 触发自动检查<br>2. `cargo clippy` 零警告<br>3. `cargo test` 全部通过<br>4. Web 端 `npm run lint` 通过 | 4 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-INFRA/Report.md) | ⏳ Pending |

---
