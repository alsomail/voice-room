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
│       └── verify-permissions.sh # 本地校验权限脚本
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
| CD 部署流水线 | 🔴 未实现 | 产物构建与部署由运维自行安排 |
