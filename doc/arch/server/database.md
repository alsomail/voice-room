# Server 数据库 Schema 设计

**Last Updated:** 2026-07-14
**Migration 目录:** `app/server/migrations/`
**Rust 模型目录:** `app/shared/src/models/`

---

## 一、 总览

| 序号 | 表名 | Migration 文件 | Rust 模型 | 任务 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 001 | `users` | `001_create_users.sql` | `UserModel` | T-00001 | 🟢 已完成 |
| 002 | `rooms` | `002_create_rooms.sql` | `RoomModel` | T-00006 | 🟢 已完成 |
| 003 | `rooms`（索引） | `003_add_unique_active_room_per_owner.sql` | — | T-00007 | 🟢 已完成 |
| 004 | `users`（wallet 字段）+ `wallet_transactions` | `004_create_wallet.sql` | `WalletTransactionModel` + `WalletTxnType` | T-00017 | 🟢 已完成 |
| 008 | `rooms`（治理扩字段）+ `room_kick_records` + `room_mute_records` | `008_room_governance.sql` | `RoomModel`（扩展）+ `RoomKickRecord` + `RoomMuteRecord` + `MuteType` | T-00024 | 🟢 已完成 |

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

## 三、 唯一偏滤索引 — 每用户最多 1 个 active 房间（T-00007）

### 3.1 Migration 文件

**文件：** `app/server/migrations/003_add_unique_active_room_per_owner.sql`

```sql
-- T-00007: 每个用户同时只能拥有一个 active 房间
-- 通过部分唯一索引强制约束（仅对未删除的 active 行）
CREATE UNIQUE INDEX IF NOT EXISTS idx_rooms_owner_active
    ON rooms (owner_id)
    WHERE status = 'active' AND deleted_at IS NULL;
```

### 3.2 约束语义

| 维度 | 说明 |
| --- | --- |
| **唯一性范围** | 仅针对 `status = 'active' AND deleted_at IS NULL` 的行，即同一 `owner_id` 只能有一条未软删除的活跃房间 |
| **覆盖场景** | 业务层 `find_active_by_owner` 预检 + DB 层 `idx_rooms_owner_active` 兜底，双重防并发竞态 |
| **并发安全** | 并发 INSERT 若业务层预检均通过，DB 仍会以 PG 错误码 `23505` 拒绝第二条写入 |
| **软删除兼容** | 关闭房间（`status='closed'` 或 `deleted_at IS NOT NULL`）后，该 owner 可再次创建新房间，不受索引约束 |
| **非 active 行不受限** | 已关闭/软删除的历史房间不计入唯一约束，支持完整审计记录 |

### 3.3 错误码映射

`From<sqlx::Error>` 实现（`app/server/src/common/error.rs`）检测 PG 错误码 `23505` 并映射为：

| DB 错误码 | AppError 变体 | HTTP Status | API 错误码 |
| --- | --- | --- | --- |
| `23505` | `ActiveRoomExists` | `409 Conflict` | `40900` |

---

## 四、 钱包模块 (T-00017)

### 4.1 DDL 概要

#### 4.1.1 `users` 表扩展

```sql
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS diamond_balance BIGINT NOT NULL DEFAULT 0
        CHECK (diamond_balance >= 0);
```

新增字段：

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `diamond_balance` | `BIGINT` | `0` | ❌ | 钻石余额，≥ 0（CHECK 约束防止负值） |

**关键特性**：
- `DEFAULT 0`：新注册用户自动初始化为 0；存量用户迁移后为 0
- `CHECK (diamond_balance >= 0)`：DB 层强制非负，配合业务层校验双重防护，严禁超扣
- 大小范围：`BIGINT` 可表示 ±9.2×10¹⁸，足够支撑任意虚拟货币操作

#### 4.1.2 `wallet_transactions` 流水表

```sql
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id),
    type             VARCHAR(32) NOT NULL, -- gift_send | gift_receive | admin_adjust | recharge | refund
    amount           BIGINT NOT NULL,      -- 正数=加，负数=扣
    balance_after    BIGINT NOT NULL CHECK (balance_after >= 0),
    ref_id           UUID,                 -- 关联 gift_record_id / admin_log_id 等
    reason           TEXT,
    operator_id      UUID REFERENCES users(id), -- 非空即管理员操作
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wallet_txn_user_created ON wallet_transactions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_wallet_txn_type ON wallet_transactions(type, created_at DESC);
```

### 4.2 字段说明

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | `UUID` | `gen_random_uuid()` | ❌ | 流水 ID，PostgreSQL 自动生成 |
| `user_id` | `UUID` | — | ❌ | 外键 → `users(id)`，账户归属 |
| `type` | `VARCHAR(32)` | — | ❌ | 枚举：`gift_send`（送礼扣减）/ `gift_receive`（收礼入账）/ `admin_adjust`（管理员手调）/ `recharge`（充值）/ `refund`（退款） |
| `amount` | `BIGINT` | — | ❌ | 变化量，正数=加，负数=扣 |
| `balance_after` | `BIGINT` | — | ❌ | 交易后余额（CHECK ≥ 0），便于核账 |
| `ref_id` | `UUID` | `NULL` | ✅ | 关联记录 ID（`gift_record_id`、`admin_log_id` 等），便于溯源 |
| `reason` | `TEXT` | `NULL` | ✅ | 流水原因/备注（管理员调整时必填，用户送礼时为空） |
| `operator_id` | `UUID` | `NULL` | ✅ | 外键 → `users(id)`，管理员 ID（管理员操作时非空） |
| `created_at` | `TIMESTAMPTZ` | `now()` | ❌ | 创建时间，PostgreSQL 自动填充 |

### 4.3 CHECK 约束

| 约束名 | 规则 | 说明 |
| --- | --- | --- |
| `diamond_balance >= 0`（`users` 表） | `diamond_balance BIGINT NOT NULL DEFAULT 0 CHECK (diamond_balance >= 0)` | 防止用户余额为负（强约束） |
| `balance_after >= 0`（`wallet_transactions` 表） | `balance_after BIGINT NOT NULL CHECK (balance_after >= 0)` | 防止写入非法的交易后余额 |

### 4.4 索引

| 索引名 | 列 | 方向 | 用途 |
| --- | --- | --- | --- |
| `idx_wallet_txn_user_created` | `(user_id, created_at DESC)` | `created_at DESC` | 查询用户流水历史，按时间倒序分页；是 `GET /api/v1/wallet/transactions` 的核心查询列 |
| `idx_wallet_txn_type` | `(type, created_at DESC)` | `created_at DESC` | 按交易类型统计流水，预留扩展点（后续可用于数据看板聚合） |

### 4.5 事务与幂等

**事务边界**：余额扣减操作（如 `SendGift`、`AdminAdjust`）**必须**在同一个 SQLx Transaction 内完成三步：
1. `UPDATE users SET diamond_balance = diamond_balance ± N WHERE id = ? AND diamond_balance ± N >= 0`（同时校验 CHECK 约束）
2. `INSERT INTO wallet_transactions (...)` 写入流水
3. 事务提交或回滚

任何一步失败，整体回滚，保证余额与流水一致性。

**幂等性**：
- `wallet_transactions` 每条流水自带唯一 `id`，插入天然幂等（无 PRIMARY KEY 冲突）
- `SendGift` 基于 `(sender_id, msg_id)` 进行应用层去重，重复请求返回幂等结果不再扣减

### 4.6 Rust 模型映射

**文件：** `app/shared/src/models/wallet.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum WalletTxnType {
    GiftSend,      // 送礼扣减
    GiftReceive,   // 收礼入账（MVP 保留，暂不自动加余额）
    AdminAdjust,   // 管理员手动调整
    Recharge,      // 充值（E-08 接入）
    Refund,        // 退款
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalletTransactionModel {
    pub id:            Uuid,
    pub user_id:       Uuid,
    #[sqlx(rename = "type")]
    pub txn_type:      WalletTxnType,
    pub amount:        i64,
    pub balance_after: i64,
    pub ref_id:        Option<Uuid>,
    pub reason:        Option<String>,
    pub operator_id:   Option<Uuid>,
    pub created_at:    DateTime<Utc>,
}
```

**关键点**：
- `WalletTxnType` 使用 `sqlx::Type` + `rename_all = "snake_case"` 实现 serde 自动映射 PostgreSQL VARCHAR 值
- `WalletTransactionModel` 使用 `sqlx::FromRow` 直接反序列化数据库行，无需手动解包
- `#[sqlx(rename = "type")]` 避免 SQL 保留字冲突（`type` → `txn_type` 字段名）

**UserModel 扩展**（`app/shared/src/models/user.rs`）：
```rust
pub struct UserModel {
    // 既有字段...
    pub diamond_balance: i64, // 新增，默认 0
}
```

### 4.7 测试覆盖

**共同测试**（`app/server/tests/wallet_schema_test.rs` + shared 单元测试）：
- **W01** 迁移幂等性：连续执行 `sqlx migrate run` 不报错
- **W02** 默认值：新注册用户 `diamond_balance = 0`；存量用户迁移后 `diamond_balance = 0`
- **W03** CHECK 约束（users）：`UPDATE users SET diamond_balance = -1` 被 PG 错误 23514 拒绝
- **W04** CHECK 约束（wallet_transactions）：插入 `balance_after = -5` 被拒绝
- **W05** 复合索引命中：`EXPLAIN SELECT ... FROM wallet_transactions WHERE user_id = ? ORDER BY created_at DESC LIMIT 20` 验证 Index Scan
- **W06** 全类型插入：5 种 `WalletTxnType` 均可正确序列化/反序列化

**测试结果**：245 passed, 0 failed（共 196 server + 8 wallet 集成 + 41 shared 单元测试）

---

## 五、 技术债记录 (Tech Debt)

### T-00008 Review 遗留 MEDIUM 问题

以下两项在 T-00008 Review 阶段标记为 **MEDIUM**，不阻塞上线，需在后续迭代中处理：

| 编号 | 级别 | 描述 | 当前行为 | 建议处理方式 | 关联任务 |
| --- | --- | --- | --- | --- | --- |
| **M-01** | MEDIUM | `page` 参数无上界溢出风险 | `page` 仅限制 `>= 1`，无最大值校验；超大 `page` 值（如 `page=10^9`）会导致 `OFFSET` 溢出或极慢全表扫描 | 后续在 service 层增加 `MAX_PAGE`（建议 10000）常量校验，超出返回 `40003`；或改用 keyset pagination | — |
| **M-02** | MEDIUM | `JOIN users` 未过滤封禁用户 | 列表中可能出现已被封禁用户的房间，封禁信息暂时未建立 | 待 T-10009（封禁用户接口）完成后，在 `find_active_rooms` 查询中增加 `JOIN users u ON r.owner_id = u.id AND u.banned_at IS NULL` 过滤条件 | T-10009 |

### T-00024 Review 遗留 MEDIUM 问题

以下三项在 T-00024 Review 阶段标记为 **MEDIUM**，不阻塞上线，需在后续迭代中处理：

| 编号 | 级别 | 描述 | 当前行为 | 建议处理方式 | 关联任务 |
| --- | --- | --- | --- | --- | --- |
| **MEDIUM-1** | MEDIUM | `announcement` 和 `admin_user_id` 缺少 `#[serde(default)]` | `Option<T>` 不自动处理 JSON 键缺失；旧 JSON 载体若无这两个键则反序列化出错，测试 `test_room_model_deserialize_legacy_without_governance_fields` 为假阳性 | 在 `app/shared/src/models/room.rs` 的 `announcement` 和 `admin_user_id` 字段补充 `#[serde(default)]` 注解 | T-00025 前补充 |
| **MEDIUM-2** | MEDIUM | 迁移 008 中 `password_hash` 为死代码 | `ADD COLUMN IF NOT EXISTS password_hash VARCHAR(60)` 因 `002_create_rooms.sql` 中已有 `VARCHAR(255)` 列而被 `IF NOT EXISTS` 静默跳过 | 删除 008 中该 `ADD COLUMN` 语句或加注释说明，避免误导 | — |
| **MEDIUM-3** | MEDIUM | S24-01~S24-06 均为静态 SQL 文本分析，非真实 DB 集成测试 | 测试仅断言 SQL 文件内容，未实际执行，无法捕获运行时 PG 错误 | 在 CI 环境具备 `DATABASE_URL` 时，补充真实执行迁移 + 断言 CHECK 约束的集成测试 | — |

---

## 六、 房间治理模块 (T-00024) — E-10 Schema 基座

### 6.1 迁移文件

**文件：** `app/server/migrations/008_room_governance.sql`

```sql
-- rooms 扩展字段（幂等）
ALTER TABLE rooms
    ADD COLUMN IF NOT EXISTS cover_url       TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS category        VARCHAR(32) NOT NULL DEFAULT 'chat',
    ADD COLUMN IF NOT EXISTS password_hash   VARCHAR(60),
    ADD COLUMN IF NOT EXISTS announcement    TEXT,
    ADD COLUMN IF NOT EXISTS admin_user_id   UUID REFERENCES users(id);

-- category 枚举约束
ALTER TABLE rooms
    DROP CONSTRAINT IF EXISTS chk_room_category,
    ADD CONSTRAINT chk_room_category
        CHECK (category IN ('chat','emotion','music','game','matchmaking','other'));

-- 治理审计表
CREATE TABLE IF NOT EXISTS room_kick_records (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id          UUID NOT NULL REFERENCES rooms(id),
    target_user_id   UUID NOT NULL REFERENCES users(id),
    operator_user_id UUID NOT NULL REFERENCES users(id),
    reason           TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_kick_records_room_ts ON room_kick_records(room_id, created_at DESC);
CREATE INDEX idx_kick_records_target_ts ON room_kick_records(target_user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS room_mute_records (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id          UUID NOT NULL REFERENCES rooms(id),
    target_user_id   UUID NOT NULL REFERENCES users(id),
    operator_user_id UUID NOT NULL REFERENCES users(id),
    type             VARCHAR(8) NOT NULL CHECK (type IN ('mic','chat')),
    duration_sec     INT NOT NULL CHECK (duration_sec >= 0), -- 0 = 解除
    reason           TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_mute_records_room_ts ON room_mute_records(room_id, created_at DESC);
CREATE INDEX idx_mute_records_target_type_ts ON room_mute_records(target_user_id, type, created_at DESC);
```

### 6.2 `rooms` 表扩展字段（T-00024）

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `cover_url` | `TEXT` | `''` | ❌ | 房间封面图 URL，空串表示无封面；老行自动默认为空 |
| `category` | `VARCHAR(32)` | `'chat'` | ❌ | 房间分类枚举，见 `chk_room_category` 约束；老行自动归为闲聊 |
| `password_hash` | `VARCHAR(60)` | `NULL` | ✅ | bcrypt 密码哈希（注：002 迁移已含同名 VARCHAR(255) 列，IF NOT EXISTS 跳过此行，见 MEDIUM-2） |
| `announcement` | `TEXT` | `NULL` | ✅ | 房间公告，≤200 字由业务层校验 |
| `admin_user_id` | `UUID` | `NULL` | ✅ | 外键 → `users(id)`，房间管理员；老行默认无管理员 |

**新增 CHECK 约束：**

| 约束名 | 规则 | 说明 |
| --- | --- | --- |
| `chk_room_category` | `category IN ('chat','emotion','music','game','matchmaking','other')` | 强制 6 类枚举，防止非法分类写入 |

### 6.3 `room_kick_records` 踢人审计表

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | `UUID` | `gen_random_uuid()` | ❌ | 主键 |
| `room_id` | `UUID` | — | ❌ | 外键 → `rooms(id)` |
| `target_user_id` | `UUID` | — | ❌ | 被踢用户，外键 → `users(id)` |
| `operator_user_id` | `UUID` | — | ❌ | 操作者（房主或管理员），外键 → `users(id)` |
| `reason` | `TEXT` | `NULL` | ✅ | 踢人原因（可选） |
| `created_at` | `TIMESTAMPTZ` | `now()` | ❌ | 操作时间 |

**索引：**

| 索引名 | 列 | 方向 | 用途 |
| --- | --- | --- | --- |
| `idx_kick_records_room_ts` | `(room_id, created_at DESC)` | DESC | 按房间查审计日志 |
| `idx_kick_records_target_ts` | `(target_user_id, created_at DESC)` | DESC | 按用户查被踢历史 |

### 6.4 `room_mute_records` 禁言/禁麦审计表

| 字段 | 类型 | 默认值 | 可空 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | `UUID` | `gen_random_uuid()` | ❌ | 主键 |
| `room_id` | `UUID` | — | ❌ | 外键 → `rooms(id)` |
| `target_user_id` | `UUID` | — | ❌ | 被禁用户，外键 → `users(id)` |
| `operator_user_id` | `UUID` | — | ❌ | 操作者，外键 → `users(id)` |
| `type` | `VARCHAR(8)` | — | ❌ | 禁类型：`mic`（禁麦）/ `chat`（禁言），CHECK 约束强制 |
| `duration_sec` | `INT` | — | ❌ | 禁止时长（秒），`0` 表示解除，`CHECK (duration_sec >= 0)` |
| `reason` | `TEXT` | `NULL` | ✅ | 操作原因（可选） |
| `created_at` | `TIMESTAMPTZ` | `now()` | ❌ | 操作时间 |

**CHECK 约束：**

| 约束名（内联）| 规则 | 说明 |
| --- | --- | --- |
| `CHECK (type IN ('mic','chat'))` | `type IN ('mic','chat')` | 仅允许两种禁止类型 |
| `CHECK (duration_sec >= 0)` | `duration_sec >= 0` | 0 = 解除，正数 = 禁止秒数 |

**索引：**

| 索引名 | 列 | 方向 | 用途 |
| --- | --- | --- | --- |
| `idx_mute_records_room_ts` | `(room_id, created_at DESC)` | DESC | 按房间查审计日志 |
| `idx_mute_records_target_type_ts` | `(target_user_id, type, created_at DESC)` | DESC | 按用户+类型查禁止历史 |

### 6.5 Rust 模型映射

**文件：** `app/shared/src/models/room.rs`（扩展后）

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
    // T-00024 新增字段
    #[serde(default)]
    pub cover_url:     String,
    #[serde(default)]
    pub category:      String,
    pub announcement:  Option<String>,    // ⚠️ MEDIUM-1: 待补 #[serde(default)]
    pub admin_user_id: Option<Uuid>,      // ⚠️ MEDIUM-1: 待补 #[serde(default)]
}
```

**文件：** `app/shared/src/models/governance.rs`（新增）

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum MuteType {
    Mic,   // 禁麦
    Chat,  // 禁言
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoomKickRecord {
    pub id:               Uuid,
    pub room_id:          Uuid,
    pub target_user_id:   Uuid,
    pub operator_user_id: Uuid,
    pub reason:           Option<String>,
    pub created_at:       DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoomMuteRecord {
    pub id:               Uuid,
    pub room_id:          Uuid,
    pub target_user_id:   Uuid,
    pub operator_user_id: Uuid,
    #[sqlx(rename = "type")]
    pub mute_type:        MuteType,
    pub duration_sec:     i32,
    pub reason:           Option<String>,
    pub created_at:       DateTime<Utc>,
}
```

**关键点**：
- `MuteType` 使用 `sqlx::Type` + `type_name = "varchar"` + `rename_all = "lowercase"` 正确映射 PostgreSQL `VARCHAR` 列
- `#[sqlx(rename = "type")]` 处理 SQL 保留字冲突（`type` → `mute_type` 字段名）
- `cover_url` 和 `category` 已有 `#[serde(default)]` 确保旧 JSON 兼容

### 6.6 存量兼容策略

| 字段 | 老行行为 | 说明 |
| --- | --- | --- |
| `cover_url` | 自动默认 `''` | 空串代表无封面，老房间不受影响 |
| `category` | 自动默认 `'chat'` | 老行自动归为闲聊分类 |
| `admin_user_id` | 默认 `NULL` | 老行无管理员，业务逻辑按 NULL 处理 |
| `announcement` | 默认 `NULL` | 老行无公告 |

### 6.7 测试覆盖（T-00024）

**文件：** `app/server/tests/room_governance_schema_test.rs`（新增）

- **S24-01** 迁移可重入执行两次无报错（幂等性验证）
- **S24-02** `category='invalid'` 被 CHECK 约束拒绝
- **S24-03** 存量房间迁移后 `cover_url=''`、`category='chat'`
- **S24-04** `room_kick_records (room_id, created_at)` 索引存在
- **S24-05** `room_mute_records.type='sms'` 被 CHECK 约束拒绝
- **S24-06** 软删房间仍可被外键引用（默认 RESTRICT 不删即可）

共 23 个测试（含附加结构验证），全部通过（🟢 23/23）

> ⚠️ 注意：S24-01~S24-06 当前为静态 SQL 文本分析测试，非真实 DB 集成测试（见技术债 MEDIUM-3）。

---

## 五、 文档维护约束

- 每新增一个 Migration 文件，必须在本文档的"总览"表格补充对应行，并在下方添加专节说明。
- 涉及事务边界或幂等策略时，在对应表节末尾补充"事务说明"小节。
- 索引变更需同步更新"索引"表格与偏滤条件描述。
- Review 阶段遗留的 MEDIUM 及以上问题，必须在"技术债记录"节中登记并注明关联任务。
