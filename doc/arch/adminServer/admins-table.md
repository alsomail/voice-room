<!--
[AI 读写规约]
1. 本文件由 DoD Agent 依据 T-10001 实现结果生成，记录 admins / admin_logs 表及相关代码结构。
2. 内容变更须同步更新 index.md 的【八、子模块索引】和【九、能力状态矩阵】。
3. 所有文件路径均为相对 monorepo 根目录的路径，须保持真实有效。
-->

# Admin Server — 管理员数据层 (T-10001)

**Last Updated:** 2026-04-19  
**Task:** T-10001 管理员表设计  
**状态:** ✅ Done (Review 通过)  
**Migration 路径:** `app/adminServer/migrations/`  
**Model 路径:** `app/shared/src/models/admin.rs`

---

## 一、admins 表结构

> 对应 Migration: `app/adminServer/migrations/001_create_admins.sql`

```sql
CREATE TABLE IF NOT EXISTS admins (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    username      VARCHAR(50)  NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    role          VARCHAR(20)  NOT NULL DEFAULT 'operator',
    display_name  VARCHAR(100),
    is_active     BOOLEAN      NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

ALTER TABLE admins ADD CONSTRAINT chk_admin_role
    CHECK (role IN ('super_admin', 'operator', 'cs', 'finance'));
```

### 字段说明

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | UUID | PRIMARY KEY, DEFAULT `gen_random_uuid()` | 主键，PostgreSQL 自动生成 |
| `username` | VARCHAR(50) | NOT NULL, UNIQUE | 登录名，全局唯一 |
| `password_hash` | VARCHAR(200) | NOT NULL | bcrypt 散列，格式 `$2b$12$…`，长度 ≥ 60 字符 |
| `role` | VARCHAR(20) | NOT NULL, DEFAULT `'operator'`, CHECK | RBAC 角色，见下方枚举 |
| `display_name` | VARCHAR(100) | NULLABLE | 管理后台展示名，可空 |
| `is_active` | BOOLEAN | NOT NULL, DEFAULT TRUE | `false` 时账号被暂停，拒绝登录 |
| `last_login_at` | TIMESTAMPTZ | NULLABLE | 最近一次成功登录时间，新账号为 NULL |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT `now()` | 行创建时间（DB 自动填充） |
| `updated_at` | TIMESTAMPTZ | NOT NULL, DEFAULT `now()` | 行更新时间（需应用层显式 SET） |

### 约束与索引

| 名称 | 类型 | 字段 | 说明 |
|------|------|------|------|
| `admins_pkey` | PRIMARY KEY | `id` | 主键索引 |
| `admins_username_key` | UNIQUE | `username` | 唯一索引（列内联定义） |
| `chk_admin_role` | CHECK | `role` | 限定四个合法角色值 |

> ⚠️ `updated_at` 无数据库触发器，UPDATE 语句须显式 `SET updated_at = now()`（待 T-10002 实现登录接口时落地）。

---

## 二、admin_logs 表结构

> 对应 Migration: `app/adminServer/migrations/002_create_admin_logs.sql`

```sql
CREATE TABLE IF NOT EXISTS admin_logs (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID         NOT NULL REFERENCES admins(id),
    action      VARCHAR(50)  NOT NULL,
    target_type VARCHAR(20),
    target_id   UUID,
    detail      JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_admin_logs_admin_id
    ON admin_logs(admin_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_admin_logs_action
    ON admin_logs(action, created_at DESC);
```

### 字段说明

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | UUID | PRIMARY KEY | 主键 |
| `admin_id` | UUID | NOT NULL, FK → `admins(id)` | 操作人 |
| `action` | VARCHAR(50) | NOT NULL | 操作类型（如 `ban_user`, `close_room`） |
| `target_type` | VARCHAR(20) | NULLABLE | 操作对象类型（`user`, `room` 等） |
| `target_id` | UUID | NULLABLE | 操作对象 ID |
| `detail` | JSONB | NULLABLE | 附加详情（结构化 JSON） |
| `ip_address` | INET | NULLABLE | 操作来源 IP |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT `now()` | 记录时间 |

### 索引

| 名称 | 字段 | 用途 |
|------|------|------|
| `idx_admin_logs_admin_id` | `(admin_id, created_at DESC)` | 按管理员倒序查询操作历史 |
| `idx_admin_logs_action` | `(action, created_at DESC)` | 按操作类型倒序查询 |

> ⚠️ `admin_id` 外键未显式声明 `ON DELETE RESTRICT`，待 T-10012 完善时补充。

---

## 三、种子数据

> 对应 Migration: `app/adminServer/migrations/003_seed_super_admin.sql`

| 字段 | 值 |
|------|-----|
| `username` | `super_admin` |
| `role` | `super_admin` |
| `display_name` | `Super Administrator` |
| `is_active` | `TRUE` |
| `password_hash` | `$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uWei` |
| 明文密码 | `admin_password_change_me` |

> ⚠️ **安全警告**：此 hash 已提交至 VCS。首次部署后须立即通过管理后台修改默认密码（在 T-10002 DoD checklist 中强制执行）。

使用 `ON CONFLICT (username) DO NOTHING` 保证幂等性。

---

## 四、AdminModel 结构体

> 文件：`app/shared/src/models/admin.rs`  
> 导出：`app/shared/src/models/mod.rs` → `pub use admin::AdminModel`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminModel {
    pub id: Uuid,                          // UUID v4
    pub username: String,                  // VARCHAR(50) UNIQUE
    pub password_hash: String,             // VARCHAR(200), bcrypt $2b$ 格式
    pub role: String,                      // VARCHAR(20) CHECK 约束
    pub display_name: Option<String>,      // VARCHAR(100) NULLABLE
    pub is_active: bool,                   // BOOLEAN
    pub last_login_at: Option<DateTime<Utc>>, // TIMESTAMPTZ NULLABLE
    pub created_at: DateTime<Utc>,         // TIMESTAMPTZ
    pub updated_at: DateTime<Utc>,         // TIMESTAMPTZ
}
```

- `sqlx::FromRow` 派生：可直接从 PgPool 查询结果映射
- `Serialize / Deserialize`：支持 JSON 序列化（API 响应）
- 9 个字段与 migration DDL 精确对齐

---

## 五、Role 枚举

> 定义：`app/adminServer/src/lib.rs` — `VALID_ADMIN_ROLES` 常量  
> 同步位置：`001_create_admins.sql` CHECK 约束 + `VALID_ADMIN_ROLES` 双层守护

| 角色值 | 说明 | 典型权限 |
|--------|------|---------|
| `super_admin` | 超级管理员 | 全部权限 |
| `operator` | 运营人员 | 用户管理、房间管理、数据统计 |
| `cs` | 客服 | 用户只读、房间管理 |
| `finance` | 财务 | 数据统计、财务操作 |

完整权限矩阵见 `doc/arch/adminServer/index.md` §六。

辅助函数：

```rust
// app/adminServer/src/lib.rs
pub fn is_valid_admin_role(role: &str) -> bool {
    VALID_ADMIN_ROLES.contains(&role)
}
```

---

## 六、bcrypt 密码策略

> 实现：`app/shared/src/crypto/password.rs`

| 项目 | 值 |
|------|-----|
| 算法 | bcrypt |
| Cost | `bcrypt::DEFAULT_COST` = 12 |
| 输出前缀 | `$2b$` 或 `$2a$` |
| 输出长度 | ≥ 60 字符，VARCHAR(200) 可完整存储 |
| 核心函数 | `hash_password(password: &str) -> Result<String, BcryptError>` |
| 验证函数 | `verify_password(password: &str, hash: &str) -> Result<bool, BcryptError>` |

---

## 七、测试覆盖

> 测试文件：`app/adminServer/src/lib.rs` `#[cfg(test)]`

| 测试 ID | 测试名 | 类型 | 覆盖点 |
|---------|--------|------|--------|
| T-10001-U01 | `admin_model_has_all_required_fields` | unit | 9 字段完整性（编译期守护） |
| T-10001-U01 | `admin_model_field_types_are_correct` | unit | 字段类型映射正确 |
| T-10001-U02 | `valid_roles_are_accepted` | unit | 四个合法角色通过 |
| T-10001-U02 | `invalid_roles_are_rejected` | unit | 9 种非法角色被拒绝 |
| T-10001-U02 | `valid_roles_count_is_exactly_four` | unit | 恰好 4 个角色 |
| T-10001-U03 | `password_hash_uses_bcrypt_format` | unit | hash 格式 `$2b$`/`$2a$` + 长度 ≥ 60 |
| T-10001-U03 | `password_hash_is_verifiable` | unit | 正确/错误密码验证 |
| T-10001-M01 | `migration_001_creates_admins_table` | unit | DDL 包含全部 9 列 |
| T-10001-M01 | `migration_001_username_has_unique_constraint` | unit | UNIQUE 关键字存在 |
| T-10001-M01 | `migration_001_role_has_check_constraint_with_all_values` | unit | CHECK + 4 角色值 |
| T-10001-M01 | `migration_001_password_hash_column_is_text_or_varchar_200` | unit | VARCHAR(200) 或 TEXT |
| T-10001-M02 | `migration_002_creates_admin_logs_table` | unit | 必要字段存在 |
| T-10001-M02 | `migration_002_admin_id_references_admins` | unit | FK `REFERENCES admins` |
| T-10001-M02 | `migration_002_has_indexes_on_admin_logs` | unit | CREATE INDEX 存在 |
| T-10001-M03 | `migration_003_seeds_default_super_admin` | unit | INSERT + super_admin |
| T-10001-M03 | `migration_003_seed_password_hash_uses_bcrypt_prefix` | unit | `$2b$` 前缀 |
| doctest | `is_valid_admin_role` doctest | doctest | 公开 API 示例 |

**合计：17 个测试（16 unit + 1 doctest），全部通过。**

---

## 八、待跟进问题（不阻塞）

| 级别 | 问题 | 建议处理时机 |
|------|------|-------------|
| MEDIUM | 默认 super_admin 密码 hash 存在 VCS，须运维强制首次改密 | T-10002 DoD checklist |
| MEDIUM | `admins.updated_at` 无自动更新触发器，需应用层显式设置 | T-10002 登录/更新接口 |
| LOW | `admin_logs.admin_id` FK 未显式声明 `ON DELETE RESTRICT` | T-10012 admin_logs 完善时 |
| LOW | `init-db.sh` 未对 `app_server_user` 显式 REVOKE admins 表权限 | 基础设施加固迭代 |

---

## 九、相关文档

- [Admin Server 架构总索引](./index.md)
- [TDS: T-10001 管理员表设计](../../tds/adminServer/T-10001.md)
- [Protocol §6.3 admins 表](../../protocol.md)
- [Protocol §6.4 admin_logs 表](../../protocol.md)
