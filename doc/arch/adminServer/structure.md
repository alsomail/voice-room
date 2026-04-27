# AdminServer 启动、配置与目录结构

## 一、目录与文件现状

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/shared/` | Workspace 共享 crate：JWT 工具、密码哈希、错误码、数据模型、新类型 | 🟢 已落地 |
| `app/adminServer/src/main.rs` | 读取配置、初始化 tracing、构建连接池 / Redis，启动 Axum Server | 🟢 已落地 |
| `app/adminServer/src/bootstrap/app.rs` | 应用初始化与依赖组装 | 🟢 已落地 |
| `app/adminServer/src/bootstrap/router.rs` | 路由注册（`/api/v1/admin/*`）与中间件链 | 🟢 已落地 |
| `app/adminServer/src/infrastructure/config.rs` | 负责 `.env`、`config/*.toml` 与环境变量覆盖的配置加载（T-10020 新增） | 🟢 已落地 |
| `app/adminServer/src/infrastructure/logging.rs` | 负责 tracing 初始化、请求日志 span | 🟢 已落地 |
| `app/adminServer/src/common/error.rs` | `AppError` 枚举、错误码映射 | 🟢 已落地 |
| `app/adminServer/src/common/result.rs` | `ApiResponse<T>` 统一成功响应结构 | 🟢 已落地 |
| `app/adminServer/src/common/middleware/` | JWT 鉴权、RBAC 权限、审计日志、request_id 中间件 | 🟢 已落地 |
| `app/adminServer/src/modules/auth/` | 管理员登录、JWT 签发、bcrypt 校验 | 🟢 已落地 |
| `app/adminServer/src/modules/user/` | 用户列表、详情、封禁/解封接口 | 🟢 已落地 |
| `app/adminServer/src/modules/wallet/` | 钱包余额调整接口 | 🟢 已落地 |
| `app/adminServer/src/modules/gift/` | 礼物 CRUD 与文件上传 | 🟢 已落地 |
| `app/adminServer/src/modules/stats/` | 数据统计接口 | 🟢 已落地 |
| `app/adminServer/src/modules/analytics/` | 用户行为查询接口 | 🟢 已落地 |
| `app/adminServer/src/modules/governance/` | 治理日志查询接口（踢人/禁言审计） | 🟢 已落地 |
| `app/adminServer/config/default.toml` | 默认配置基线，包含 `[app]` `[server]` `[database]` `[jwt]` `[log]` `[storage]` 6 个章节（T-10020 新增） | 🟢 已落地 |
| `app/adminServer/config/{dev,test,staging,prod}.toml` | 各 profile 差异化配置（T-10020 新增） | 🟢 已落地 |

## 二、当前启动流程（含 T-10020 config 多 profile 体系）

### 启动阶段详解

1. **配置加载**（`AdminSettings::load()`，T-10020 新增）：
   - 加载链：`.env` → 解析 `ADMIN_PROFILE`（优先级 `ADMIN_PROFILE > ADMIN_ENV > ADMIN__ENVIRONMENT > "dev"` 默认）→ 白名单校验 `{dev,test,staging,prod}` → 默认值 → `default.toml` → `{profile}.toml` → ENV 覆盖
   - 敏感字段 fail-fast：`DATABASE_URL` 必填，`ADMIN_JWT_SECRET`（或 `JWT_SECRET` 回落）必填，`REDIS_URL` dev 允许缺失（NoopEventPublisher），其他 profile 必填
   - 新支持 ENV 覆盖：`ADMIN__SERVER__*`、`ADMIN__LOG__*`、`ADMIN__JWT__*`、`ADMIN__DATABASE__*`、`ADMIN__STORAGE__*`
   - 兼容别名：`PORT` → `server.port`、`GIFT_UPLOAD_DIR` → `storage.gift_upload_dir`、`ADMIN_ENV` 别名
   - 启动摘要日志：含 profile、host、port、DB/Redis 凭据脱敏、JWT 密钥长度（永不打印明文）

2. `init_tracing()` 按 `log.format` 选择 JSON 或普通文本日志。

3. `create_pool()` 初始化 SQLx PostgreSQL 连接池；通过 `voice_room_shared::migrate::run_migrations_with_table` 以自定义登记表 `_sqlx_admin_migrations` 运行迁移，与 AppServer 的 `_sqlx_app_migrations` 物理共存、逻辑隔离（详见 [T-0000M](../../tds/infra/T-0000M.md) / [ADR-0001](../../adr/ADR-0001-migration-table-isolation.md)）。

4. Redis 连接初始化（若 `redis_url` 为 None，则装 `NoopEventPublisher`；否则 `RedisEventPublisher`）。

5. `AppState::new(...)` 组装依赖，构造各业务 service。

6. `build_app(state)` 注册路由、注入中间件链（JWT → RBAC → 审计 → request_id）。

7. Server 监听 `settings.server.bind_addr()`（AdminServer 默认 `3001`），并支持优雅退出。

### T-10020 fail-fast 行为

当启动失败时（缺 `DATABASE_URL`、缺 `ADMIN_JWT_SECRET`、无效 profile、非 dev profile 缺 REDIS_URL 等），`main.rs` 捕获 anyhow Error 并输出：
```
CONFIG ERROR: <错误正文>
```
然后 `std::process::exit(78)`（EX_CONFIG，便于 preflight 脚本定位）。

## 三、配置加载与多 profile 体系（T-10020）

### 3.1 配置来源与加载链

| 来源 | 说明 | 涵盖 T-10020 |
| --- | --- | --- |
| `.env.example` | 提供 `ADMIN_PROFILE=dev`（主入口）、`DATABASE_URL`、`ADMIN_JWT_SECRET`、`REDIS_URL`、日志配置等本地模板 | ✅ |
| `config/default.toml` | 默认配置基线，包含 `[app]` `[server]` `[database]` `[jwt]` `[log]` `[storage]` 6 个章节，所有敏感字段由 ENV 注入 | ✅ |
| `config/dev.toml` | Dev profile 差异：`log.level=debug`；`port=3001` | ✅ |
| `config/test.toml` | Test profile 差异：`server.port=4001`、`jwt.expire_secs=3600` | ✅ |
| `config/staging.toml` | **Staging profile 新增**：`environment="staging"`、`log.format=json` | ✅ |
| `config/prod.toml` | Prod profile 差异：`server.port=3001`、`log.format=json`、`database.max_connections=50` | ✅ |
| 环境变量 | 支持 `ADMIN_PROFILE` / `ADMIN_ENV`（向后兼容）、`ADMIN__SERVER__*`、`ADMIN__LOG__*`、`ADMIN__JWT__*`、`ADMIN__DATABASE__*`、`ADMIN__STORAGE__*` 全系列覆盖，优先级最高；兼容别名 `PORT` 和 `GIFT_UPLOAD_DIR` | ✅ |

### 3.2 Profile 白名单与默认值

| Profile | 环境变量值 | `app.environment` | `log.level` | `server.port` | 特殊行为 |
| --- | --- | --- | --- | --- | --- |
| `dev` | `ADMIN_PROFILE=dev` | `"dev"` | `debug` | `3001` | REDIS_URL 缺失时保留 None、装 NoopEventPublisher + WARN（0 回归），其余必填 |
| `test` | `ADMIN_PROFILE=test` | `"test"` | `info` | `4001` | 端口独立避免与 AppServer test=4000 冲突 |
| `staging` | `ADMIN_PROFILE=staging` | `"staging"` | `info` | `3001` | 全部敏感字段严格必填 |
| `prod` | `ADMIN_PROFILE=prod` | `"prod"` | `info` | `3001` | 全部敏感字段严格必填，log 强制 JSON |

> 若 `ADMIN_PROFILE` 不在白名单，启动立即失败：`CONFIG ERROR: invalid ADMIN_PROFILE='xxx'; expected one of [dev,test,staging,prod]` → 退出码 78

### 3.3 必填字段与 fail-fast 契约

| 必填字段 | 校验点 | 缺失/空时 | 退出码 | 错误消息前缀 |
| --- | --- | --- | --- | --- |
| `DATABASE_URL` | **所有 profile** | 启动失败 | 78 | `CONFIG ERROR: DATABASE_URL must be set` |
| `ADMIN_JWT_SECRET` 或 `JWT_SECRET` | **所有 profile** | 启动失败；占位符 `change-me-in-production` 亦拒绝 | 78 | `CONFIG ERROR: ADMIN_JWT_SECRET (or JWT_SECRET fallback) must be set` 或 `still equals the placeholder` |
| `REDIS_URL` | dev 允许缺失（WARN + NoopEventPublisher）；**其他 profile 必填** | dev 装 Noop；非 dev 启动失败 | 78 (非dev) | `CONFIG ERROR: REDIS_URL must be set for non-dev profile` |

**D-A1 决策**：AdminServer dev profile 与 AppServer dev profile 差异：
- AppServer dev：缺失 REDIS_URL 时**回退内置 URL** `redis://127.0.0.1:6379`（`allow_redis_fallback=true`）
- AdminServer dev：缺失 REDIS_URL 时**保留 None + WARN**，main.rs 装 `NoopEventPublisher`（0 回归当前行为，避免本地 redis 依赖）

## 四、配置文件字段冻结表（§2.3 完整骨架）

### 默认配置 `default.toml`（profile-agnostic 基线）

```toml
[app]
name = "voice-room-admin-server"
environment = "dev"               # 会被 {profile}.toml / ADMIN_PROFILE 覆盖

[server]
host = "0.0.0.0"
port = 3001

[database]
# DSN 完全由 DATABASE_URL 注入，禁止在 toml 写入明文凭据
max_connections = 10
connect_timeout_secs = 5

[jwt]
# 密钥由 ADMIN_JWT_SECRET 或 JWT_SECRET 注入
expire_secs = 86400

[log]
level = "info"
format = "json"
service_name = "voice-room-admin-server"

[storage]
# 礼物上传目录；可被 GIFT_UPLOAD_DIR 或 ADMIN__STORAGE__GIFT_UPLOAD_DIR 覆盖
gift_upload_dir = "./uploads/gifts"

[redis]
# URL 完全由 REDIS_URL 注入；预留章节用于 future override
```

### 分 Profile 差异表

| 配置项 | `default.toml` | `dev.toml` | `test.toml` | `staging.toml` | `prod.toml` |
|---|---|---|---|---|---|
| `app.environment` | `"dev"` | — | `"test"` | `"staging"` | `"prod"` |
| `log.level` | `"info"` | `"debug"` | — | — | — |
| `log.format` | `"json"` | — | — | — | — |
| `server.port` | `3001` | — | `4001` | — | `3001` |
| `database.max_connections` | `10` | — | — | — | `50` |
| `database.connect_timeout_secs` | `5` | — | — | — | `10` |
| `jwt.expire_secs` | `86400` | — | `3600` | — | — |

**说明**：
- 空白 `—` 表示沿用 `default.toml` 值
- `dev.toml` 最轻量（仅 `log.level=debug`）
- `test.toml` 独立端口 4001 避免与 AppServer test 冲突，JWT token 短期 3600s 便于测试
- `staging.toml` 生产级配置（环境标签 + JSON 日志）
- `prod.toml` 最严格（大连接池 50、长超时 10s、JSON 日志）
- **AdminServer 独有**：`[storage]` 章节（存放 `gift_upload_dir`），AppServer 无此配置

## 五、当前测试面

- `src/bootstrap/` 中的集成测试对 `build_app()` 发起 HTTP 请求。
- `src/modules/*/service.rs` 包含单元测试。
- **T-10020 新增** `tests/admin_settings_load_test.rs`：集成测试 5 cases（I1~I5），覆盖各 profile 加载、ENV override、必填字段校验、D-A1 契约（dev REDIS_URL 缺失）。
- **T-10020 新增** `src/infrastructure/config.rs` 单元测试 27 cases（U1.1~U1.5 profile 解析 / U2.1~U2.5 加载链优先级 / U3.1~U3.5 敏感字段 / U4.1~U4.6 ENV override + 兼容别名 / U5.1~U5.4 日志脱敏 + U6.1~U6.2 NoopEventPublisher 容忍）。
- 测试覆盖点：
  - Profile 白名单校验（dev/test/staging/prod 有效，others 拒绝）
  - 加载链优先级（default → profile.toml → ENV，ENV 最高）
  - `require_admin_jwt_secret()` 双源（ADMIN_JWT_SECRET 优先 > JWT_SECRET 回落）
  - 敏感字段 fail-fast（缺失 / 空白 / 占位符 → exit 78）
  - ENV 覆盖与兼容别名（PORT / GIFT_UPLOAD_DIR / ADMIN_ENV）
  - 启动摘要日志脱敏（密钥明文 0 命中，URL 移除 userinfo）
  - D-A1 契约（dev 缺 REDIS_URL → None + WARN；非 dev → exit 78）
- **当前通过测试数**：474 个（库级 `cargo test -p voice-room-admin-server --lib`），加集成 5 个（`cargo test -p voice-room-admin-server --test admin_settings_load_test`），**`cargo clippy` 零警告**（T-10020 增加了 27 个新增单元测试 + 5 个集成测试，0 回归）

## 六、与 AppServer T-00040 对称差异表（§2.8）

| 维度 | AppServer (T-00040) | AdminServer (T-10020) | 差异说明 |
|---|---|---|---|
| profile 入口 | `APP_PROFILE` | `ADMIN_PROFILE` | 命名空间隔离 ✅ |
| 向后兼容别名 | `APP_ENV` `APP__ENVIRONMENT` | `ADMIN_ENV` `ADMIN__ENVIRONMENT` | 保留兼容 |
| ENV override 前缀 | `APP__` | `ADMIN__` | 隔离一致 ✅ |
| 端口（dev/test/staging/prod） | 3000 / 4000 / 3000 / 3000 | 3001 / 4001 / 3001 / 3001 | AdminServer +1 避免冲突 ✅ |
| REDIS_URL 缺失 dev 行为 | 回退 `redis://127.0.0.1:6379` | 保留 None + NoopEventPublisher + WARN | D-A1 决策：0 回归当前行为 |
| JWT_SECRET 源 | `JWT_SECRET` 单一 | `ADMIN_JWT_SECRET` 优先 > `JWT_SECRET` 回落 | 前瞻性拆分（真正独立 task） |
| storage 配置 | 无 | `[storage]` 章节 + `gift_upload_dir` | AdminServer 独有（礼物上传） |
| config 加载器 | 自研 Rust `load()` | 自研 Rust `load()` | 同源设计，避免 third-party 异构 |
| 敏感字段脱敏 | 无明文 JWT secret 日志；URL 移除 userinfo | 同左 | 一致 ✅ |

## 七、结论

AdminServer 启动与配置体系（T-10020）与 AppServer T-00040 **完全对称**，引入 `ADMIN_PROFILE` 多环境（dev/test/staging/prod）分层与 fail-fast 机制，满足模块 9 E2E 测试基建需求。关键差异（端口隔离、REDIS_URL dev 容忍、storage 独有配置）已冻结于 TDS §2.8，保 0 回归前提下最大程度对称化。
