# Server 数据库 Schema 设计

**Last Updated:** 2025-01-31
**Migration 目录:** `app/server/migrations/`
**Rust 模型目录:** `app/shared/src/models/`

---

## 一、 总览

| 序号 | 表名 | Migration 文件 | Rust 模型 | 任务 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 001 | `users` | `001_create_users.sql` | `UserModel` | T-00001 | 🟢 已完成 |
| 002 | `rooms` | `002_create_rooms.sql` | `RoomModel` | T-00006 | 🟢 已完成 |

---

## 二、 `rooms` 表（T-00006）

### 2.1 DDL 概要

```sql
CREATE TABLE IF NOT EXISTS rooms (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id      UUID         NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    title         VARCHAR(30)  NOT NULL,
    room_type     VARCHAR(20)  NOT NULL DEFAULT 'normal',
    member_count  INT          NOT NULL DEFAULT 0,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    password_hash VARCHAR(255),
    max_members   INT          NOT NULL DEFAULT 50,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ,

    CONSTRAINT chk_rooms_title_length          CHECK (char_length(title) BETWEEN 1 AND 30),
    CONSTRAINT chk_rooms_room_type             CHECK (room_type IN ('normal', 'password', 'paid')),
    CONSTRAINT chk_rooms_status                CHECK (status IN ('active', 'closed')),
    CONSTRAINT chk_rooms_member_count_non_negative CHECK (member_count >= 0),
    CONSTRAINT chk_rooms_member_count_le_max   CHECK (member_count <= max_members),
    CONSTRAINT chk_rooms_max_members_positive  CHECK (max_members > 0)
);
```

### 2.2 字段说明

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | `UUID` | `gen_random_uuid()` | ❌ | 主键，PostgreSQL 自动生成 UUID v4 |
| `owner_id` | `UUID` | — | ❌ | 外键 → `users(id)` ON DELETE RESTRICT，防止删除有房间的用户 |
| `title` | `VARCHAR(30)` | — | ❌ | 房间标题，1–30 字符（DB CHECK 约束） |
| `room_type` | `VARCHAR(20)` | `'normal'` | ❌ | 枚举：`normal` / `password` / `paid` |
| `member_count` | `INT` | `0` | ❌ | 当前在线人数，≥ 0 且 ≤ `max_members` |
| `status` | `VARCHAR(20)` | `'active'` | ❌ | 枚举：`active` / `closed` |
| `password_hash` | `VARCHAR(255)` | `NULL` | ✅ | bcrypt 哈希，仅 `room_type='password'` 时设置 |
| `max_members` | `INT` | `50` | ❌ | 房间人数上限，> 0 |
| `created_at` | `TIMESTAMPTZ` | `NOW()` | ❌ | 创建时间，PostgreSQL 自动填充 |
| `updated_at` | `TIMESTAMPTZ` | `NOW()` | ❌ | 最后更新时间，业务层负责维护 |
| `deleted_at` | `TIMESTAMPTZ` | `NULL` | ✅ | 软删除时间戳；`NULL` 表示未删除 |

### 2.3 CHECK 约束

| 约束名 | 规则 | 说明 |
| --- | --- | --- |
| `chk_rooms_title_length` | `char_length(title) BETWEEN 1 AND 30` | 标题非空且不超过 30 字符 |
| `chk_rooms_room_type` | `room_type IN ('normal', 'password', 'paid')` | 防止非法类型写入 |
| `chk_rooms_status` | `status IN ('active', 'closed')` | 防止非法状态写入 |
| `chk_rooms_member_count_non_negative` | `member_count >= 0` | 人数不能为负 |
| `chk_rooms_member_count_le_max` | `member_count <= max_members` | 实际人数不超过上限 |
| `chk_rooms_max_members_positive` | `max_members > 0` | 上限必须为正数 |

### 2.4 索引

| 索引名 | 列 | 方向 | 偏滤条件 | 用途 |
| --- | --- | --- | --- | --- |
| `idx_rooms_status_created_at` | `(status, created_at)` | `created_at DESC` | `WHERE deleted_at IS NULL` | 房间列表按状态+时间查询，自动排除软删除行 |
| `idx_rooms_owner_id` | `owner_id` | — | — | 查询用户拥有的所有房间 |
| `idx_rooms_member_count` | `member_count` | `DESC` | `WHERE deleted_at IS NULL` | 热度排序（在线人数降序），自动排除软删除行 |

> **偏滤索引（Partial Index）说明**：`idx_rooms_status_created_at` 和 `idx_rooms_member_count` 均携带 `WHERE deleted_at IS NULL`，确保软删除行不进入索引，减少索引体积并提升高频查询性能。

### 2.5 软删除策略

- 删除操作设置 `deleted_at = NOW()`，不物理删除行。
- 所有业务查询必须携带 `WHERE deleted_at IS NULL`（偏滤索引已自动覆盖）。
- 外键 `owner_id REFERENCES users(id) ON DELETE RESTRICT` 确保软删除房间数据保留引用完整性。

### 2.6 Rust 模型映射

**文件：** `app/shared/src/models/room.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoomModel {
    pub id:            Uuid,
    pub owner_id:      Uuid,
    pub title:         String,
    pub room_type:     String,
    pub member_count:  i32,
    pub status:        String,
    pub password_hash: Option<String>,
    pub max_members:   i32,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
    pub deleted_at:    Option<DateTime<Utc>>,
}
```

- `sqlx::FromRow` 直接将 PostgreSQL 行映射到 struct，无需手动解包。
- `password_hash` 和 `deleted_at` 为 `Option<T>`，对应数据库可空列。
- **测试覆盖：** 29 个单元测试，含序列化/反序列化、软删除字段可空性、Migration SQL 内容逐项断言（`cargo test -p shared`全通过）。

---

## 三、 文档维护约束

- 每新增一个 Migration 文件，必须在本文档的"总览"表格补充对应行，并在下方添加专节说明。
- 涉及事务边界或幂等策略时，在对应表节末尾补充"事务说明"小节。
- 索引变更需同步更新"索引"表格与偏滤条件描述。
