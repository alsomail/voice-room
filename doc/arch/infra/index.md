# 工程基建架构（Infra）

> 开发环境容器编排、数据库初始化权限隔离、CI 自动化流水线。  
> 对应任务：T-0000A、T-0000C、T-0000D。

---

## 目录

- [一、目录结构](#一目录结构)
- [二、开发环境容器编排（T-0000A/C）](#二开发环境容器编排t-0000ac)
- [三、数据库权限隔离（T-0000C）](#三数据库权限隔离t-0000c)
- [四、CI 流水线（T-0000D）](#四ci-流水线t-0000d)
- [五、环境变量说明](#五环境变量说明)
- [六、能力矩阵](#六能力矩阵)

---

## 一、目录结构

```text
.
├── docker-compose.yml            # 开发环境 PostgreSQL + Redis
├── .env.example                  # 环境变量模板（含密码/JWT secret）
├── scripts/
│   └── dev/
│       ├── init-db.sh            # PG 容器首次启动时的 Role/DB 初始化
│       ├── grant-permissions.sql # 最小权限隔离 SQL
│       ├── verify-permissions.sh # 本地校验权限脚本
│       ├── seed-e2e.sh           # E2E Seed 幂等脚本（T-0000G）
│       ├── seed-e2e.sql          # E2E 数据幂等插入 SQL（T-0000G）
│       ├── reset-e2e.sh          # E2E 数据清空脚本（T-0000G）
│       └── preflight.sh          # 5 端健康检查脚本（T-0000G）
├── app/shared/src/bin/
│       └── sign_jwt.rs           # E2E 用 JWT 签发 CLI（T-0000G）
└── .github/
    └── workflows/
        └── ci.yml                # Rust Lint + Test 流水线
```

---

## 二、开发环境容器编排（T-0000A/C）

### 服务列表

| 服务 | 镜像 | 本地端口 | 说明 |
|------|------|----------|------|
| `postgres` | `postgres:16-alpine` | `127.0.0.1:5432` | 主数据库，仅绑定本地 |
| `redis` | `redis:7-alpine` | `127.0.0.1:6379` | 发布订阅 / 会话 Cache |

> **端口安全**：所有端口绑定 `127.0.0.1`，不对外暴露。

### 初始化流程

```
docker compose up -d postgres
  └─► 首次创建 volume 时执行 docker-entrypoint-initdb.d/
        └─► init-db.sh
              ├─ 读取 $APP_SERVER_PASS（来自 .env）
              ├─ 读取 $ADMIN_SERVER_PASS（来自 .env）
              ├─ 创建 app_server_user / admin_server_user Role
              ├─ 创建 voiceroom 数据库
              └─ \c voiceroom → 执行 grant-permissions.sql
```

> **幂等说明**：`init-db.sh` 只在 volume 首次创建时执行一次。重建权限请运行 `verify-permissions.sh`。

---

## 三、数据库权限隔离（T-0000C）

### 角色权限矩阵

| Role | 权限 | 说明 |
|------|------|------|
| `app_server_user` | `SELECT, INSERT, UPDATE` on ALL TABLES / SEQUENCES | 业务服务（不允许 DELETE、DDL） |
| `admin_server_user` | 全部表权限（含 `DELETE`） | 管理后台 |

### SQL 关键语句（`grant-permissions.sql`）

```sql
-- 存量表权限
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO app_server_user;
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO app_server_user;

-- 新建表自动继承（避免 DDL 后需重新授权）
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE ON TABLES TO app_server_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO app_server_user;

GRANT ALL ON ALL TABLES IN SCHEMA public TO admin_server_user;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO admin_server_user;
```

---

## 四、E2E 测试基建脚本（T-0000G）

**关联 TDS**：[T-0000G Seed/Reset/Preflight 三件套脚本](../../tds/infra/T-0000G.md)

### 核心脚本

| 脚本 | 位置 | 用途 | 入参 | 安全约束 |
|------|------|------|------|---------|
| `seed-e2e.sh` | `scripts/dev/seed-e2e.sh` | 幂等创建 E2E 测试用户/房间/Token，回填到 `.seed-output.env` | `E2E_PROFILE`、`JWT_SECRET`、PG 连接参数 | `E2E_PROFILE=local` 强制，非 local 退出码 21 |
| `seed-e2e.sql` | `scripts/dev/seed-e2e.sql` | 幂等 SQL 语句，使用 `ON CONFLICT DO UPDATE` 确保重复执行结果一致 | UUIDv5 ID、Phone、Token 等（由 wrapper 注入） | 无直接执行，由 wrapper `psql -v` 传参 |
| `reset-e2e.sh` | `scripts/dev/reset-e2e.sh` | 清空 E2E 测试数据（用户/房间/Token），不影响业务表结构 | `E2E_PROFILE`、PG+Redis 连接参数 | `E2E_PROFILE=local` 强制，非 local 退出码 21 |
| `preflight.sh` | `scripts/dev/preflight.sh` | 5 端健康检查（Postgres/Redis/AppServer/AdminServer/Web），任一失败 2s 内输出彩色定位 | `E2E_PROFILE`、服务 URL、连接参数 | 串行检查、独立退出码 11-15、CI=1 关闭颜色 |

### sign-jwt CLI 工具（T-0000G）

**位置**：`app/shared/src/bin/sign_jwt.rs`

**用途**：为 E2E Seed 脚本签发 JWT Token（AppClaims / AdminClaims）

**使用方式**：

```bash
# 签发 AppClaims（C 端用户）
sign-jwt --sub <uuid> --role user --ttl <seconds>
  → iss="voiceroom", exp=iat+<ttl>

# 签发 AdminClaims（B 端管理员）
sign-jwt --sub <uuid> --role admin --ttl <seconds>
  → iss="voiceroom-admin", role=super_admin, exp=iat+<ttl>

# 支持的 admin role：admin, op, cs, fin
# 映射关系：admin→super_admin, op→operator, cs→cs, fin→finance

# 计算 E2E 命名空间 UUIDv5
sign-jwt --uuid5 <name>
  → E2E_NS = 9b3e0c6a-1ec1-4d3f-93d4-e2e000000000
```

**环境变量**：
- `JWT_SECRET`（必需）：从 env 读取，永不 echo（安全考量）

**退出码**：
- `0`：成功
- `2`：入参错误
- `3`：缺少 `JWT_SECRET`
- `4`：签发失败

---

## 四、CI 流水线（T-0000D）

**文件**：`.github/workflows/ci.yml`  
**触发条件**：`push` 到 `main`，或任意 PR

### Job 结构

```
Rust Lint + Test (ubuntu-latest)
 ├─ actions/checkout
 ├─ dtolnay/rust-toolchain@stable  # 读 rust-toolchain.toml (stable 1.95)
 ├─ Swatinem/rust-cache            # 缓存 ~/.cargo + target/
 ├─ cargo fmt --all -- --check     # 格式检查（timeout: 5min）
 ├─ cargo clippy --workspace --all-targets -- -D warnings  # Lint（timeout: 15min）
 └─ cargo test --workspace         # 全量测试（timeout: 30min）
```

> **无构建产物**：CI 不执行 `--release` 构建，产物构建由后续 deploy workflow 负责。  
> **Lint 零警告**：`-D warnings` 强制执行，警告即失败。

---

## 五、环境变量说明

参见 `.env.example`：

| 变量 | 说明 | 默认值（仅 Dev）|
|------|------|----------------|
| `POSTGRES_PASSWORD` | PG superuser 密码 | `postgres_dev_pass` |
| `APP_SERVER_PASS` | `app_server_user` 密码 | `app_server_pass` |
| `ADMIN_SERVER_PASS` | `admin_server_user` 密码 | `admin_server_pass` |
| `APP_JWT_SECRET` | App Server JWT 签名密钥 | `your-app-jwt-secret-here` |
| `ADMIN_JWT_SECRET` | Admin Server JWT 签名密钥 | `your-admin-jwt-secret-here` |

> ⚠️ **生产环境**：所有密钥必须从 Vault / Secrets Manager 注入，严禁使用默认值。

---

## 六、能力矩阵

| 能力 | 状态 | 说明 |
|------|------|------|
| PG + Redis 容器编排 | 🟢 完成 | `docker-compose.yml` |
| DB Role 初始化脚本 | 🟢 完成 | `init-db.sh`（密码从环境变量读取）|
| 最小权限隔离 SQL | 🟢 完成 | `grant-permissions.sql`（含 ALTER DEFAULT PRIVILEGES）|
| 权限校验脚本 | 🟢 完成 | `verify-permissions.sh` |
| CI Lint + Test | 🟢 完成 | `.github/workflows/ci.yml` |
| 环境变量模板 | 🟢 完成 | `.env.example` |
| **E2E Seed 幂等脚本** | **🟢 完成** | **T-0000G：`seed-e2e.sh` + `seed-e2e.sql`（创建测试用户/房间/Token）** |
| **E2E Reset 清理脚本** | **🟢 完成** | **T-0000G：`reset-e2e.sh`（清空测试数据，profile-guard 防非 local）** |
| **E2E Preflight 健康检查** | **🟢 完成** | **T-0000G：`preflight.sh`（5 端检查，2s 超时，彩色输出）** |
| **sign-jwt JWT 签发 CLI** | **🟢 完成** | **T-0000G：`app/shared/src/bin/sign_jwt.rs`（支持 AppClaims/AdminClaims、UUIDv5 计算）** |
| **E2E envLoader（单一加载源）** | **🟢 完成** | **T-0000H：`tests/scripts/support/envLoader.ts`（24 字段加载链、MissingEnvError/InvalidProfileError、退出码 78 冻结）** |
| **E2E globalSetup（启动编排 5 步）** | **🟢 完成** | **T-0000H：`tests/scripts/support/globalSetup.ts`（Step1-5：env 加载→preflight→seed→writeProcessEnv→DotFile；preflight 失败不调 seed；退出码 11-15/21-24 透传）** |
| **E2E globalTeardown（幂等清理）** | **🟢 完成** | **T-0000H：`tests/scripts/support/globalTeardown.ts`（profile≠local skip、E2E_RESET=0 skip、reset 失败仅 warn）** |
| **E2E fixtures（五道防线）** | **🟢 完成** | **T-0000H：`tests/scripts/support/fixtures.ts`（L1 prod.example=0 / L2 envLoader warn / L3 fixtures auto skip / L4 写 fixture skip / L5 config grep @prod-safe）** |
| **playwright E2E config** | **🟢 完成** | **T-0000H：`playwright.config.ts`（globalSetup/Teardown 接入、grep @prod-safe 条件、use.baseURL lazy 读 ADMIN_WEB_URL）** |
| **playwright unit config** | **🟢 完成** | **T-0000H：`playwright.unit.config.ts`（单测专用，隔离生产 setup）** |
| **TypeScript strict config** | **🟢 完成** | **T-0000H：`tsconfig.json`（scope=support/，tsc --noEmit 0 错误）** |
| **npm scripts 一键命令** | **🟢 完成** | **T-0000I：`package.json` scripts（6 条一键命令 + cross-env 跨平台、`e2e:local/staging/prod-smoke` + `db:seed/reset` + `preflight`、退出码透传 11~15/21~24）** |
| CD 部署流水线 | 🔴 未实现 | 产物构建与部署由运维自行安排 |
