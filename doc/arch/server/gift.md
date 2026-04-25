# 🎁 Gift 模块架构设计

**最后更新**：2025-06-27 (T-00020 Review Round 2 通过)  
**覆盖 Tasks**：T-00019 (配置表 + 列表 API)、T-00020 (SendGift 事务 + 广播)、T-00021 (榜单 API，设计中)

---

## 一、模块概览

Gift 模块位于 `app/server/src/modules/gift/`，提供：
- ✅ **配置管理** — `gifts` 表 + 8 款 MVP 礼物种子数据（T-00019）
- ✅ **列表查询** — `GET /api/v1/gifts/list`，国际化支持，60s 进程内存缓存（T-00019）
- ✅ **发送礼物** — WS 信令 `SendGift`，强事务编排，基于 msg_id 幂等（T-00020）
- ✅ **实时广播** — `GiftReceived` 房间广播 + `BalanceUpdated` 发送者推送（T-00020）
- ✅ **魅力榜单** — Redis ZSet 日/周榜更新、排名查询（T-00021，设计中）
- ✅ **魅力值累积** — 接收者 `users.charm_balance` 字段更新

---

## 二、核心数据模型

### 2.1 数据库表结构

**`gifts` 表** — T-00019
```sql
CREATE TABLE IF NOT EXISTS gifts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code         VARCHAR(32) UNIQUE NOT NULL,        -- 稳定标识如 'rose_01'
    name_en      VARCHAR(128) NOT NULL,
    name_ar      VARCHAR(128) NOT NULL,
    icon_url     VARCHAR(512),
    price        BIGINT NOT NULL CHECK (price >= 1),
    tier         SMALLINT NOT NULL CHECK (tier BETWEEN 1 AND 5),  -- 1=entry, 5=premium
    effect_level SMALLINT DEFAULT 1,                 -- 1=none, 2=slot, 3=bottom, 4=fullscreen, 5=fullscreen+border
    animation_url VARCHAR(512),
    sort_order   INT DEFAULT 0,                      -- 同 tier 内排序
    is_active    BOOLEAN DEFAULT true,
    is_deleted   BOOLEAN DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 查询加速索引（仅活跃礼物）
CREATE INDEX IF NOT EXISTS idx_gifts_active_order 
  ON gifts(tier, sort_order) WHERE is_active AND NOT is_deleted;
```

**`users` 表扩展** — T-00020（新增魅力值字段）
```sql
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS charm_balance BIGINT NOT NULL DEFAULT 0 CHECK (charm_balance >= 0);
```

**`gift_records` 表** — T-00020（礼物赠送记录）
```sql
CREATE TABLE IF NOT EXISTS gift_records (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id    UUID NOT NULL REFERENCES users(id),
    receiver_id  UUID NOT NULL REFERENCES users(id),
    room_id      UUID NOT NULL REFERENCES rooms(id),
    gift_id      UUID NOT NULL REFERENCES gifts(id),
    count        INT NOT NULL CHECK (count >= 1 AND count <= 9999),
    total_price  BIGINT NOT NULL CHECK (total_price >= 1),
    msg_id       VARCHAR(64) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- 幂等约束：同一发送者相同 msg_id 仅允许一条记录
    UNIQUE (sender_id, msg_id)
);

CREATE INDEX IF NOT EXISTS idx_gift_records_receiver_created 
  ON gift_records(receiver_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_gift_records_room_created 
  ON gift_records(room_id, created_at DESC);
```

### 2.2 Rust 数据模型

**`GiftModel`** — 来自 `app/shared/src/models/gift.rs`
```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct GiftModel {
    pub id: Uuid,
    pub code: String,
    pub name_en: String,
    pub name_ar: String,
    pub icon_url: Option<String>,
    pub price: i64,
    pub tier: i16,
    pub effect_level: i16,
    pub animation_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**`GiftRecordModel`** — 来自 `app/shared/src/models/gift_record.rs`
```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct GiftRecordModel {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
    pub room_id: Uuid,
    pub gift_id: Uuid,
    pub count: i32,
    pub total_price: i64,
    pub msg_id: String,
    pub created_at: DateTime<Utc>,
}
```

**`UserModel` 扩展** — 新增 `charm_balance` 字段
```rust
pub struct UserModel {
    pub id: Uuid,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub diamond_balance: i64,
    pub charm_balance: i64,  // ← T-00020 新增
    pub vip_level: i16,
    pub is_banned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 三、模块文件结构

```
app/server/src/modules/gift/
├── mod.rs                  # 模块入口，导出 ranking / send_gift 子模块
├── ranking.rs              # Redis ZSet 榜单操作（T-00021）
├── send_gift.rs            # SendGift 信令处理器 + GiftSendService 事务编排
└── (repository.rs)         # TBD：Gift 数据层抽象

app/server/migrations/
├── 005_create_gifts.sql    # T-00019：gifts 表 + 8 款种子数据
└── 006_create_gift_records.sql  # T-00020：gift_records 表 + users.charm_balance
```

---

## 四、T-00019 礼物配置与列表 API

### 4.1 HTTP 接口：`GET /api/v1/gifts/list`

**请求**
```http
GET /api/v1/gifts/list
Accept-Language: ar  # 默认阿拉伯语，支持 en/en-US 等
```

**响应 (code=0 成功)**
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "uuid",
        "code": "rose_01",
        "name": "وردة",
        "icon_url": "https://...",
        "price": 10,
        "tier": 1,
        "effect_level": 2,
        "animation_url": "https://...",
        "sort_order": 0
      },
      // ... 更多礼物
    ],
    "version": "1720000000000"  // 缓存版本时间戳
  }
}
```

### 4.2 国际化设计

- **Accept-Language 解析**：大小写不敏感，`en/en-US/en-GB` 等映射到 `"en"`，其余默认 `"ar"`
- **响应字段**：`name` 根据语言选择 `name_en` 或 `name_ar`
- **过滤条件**：仅返回 `is_active=true AND is_deleted=false`
- **排序策略**：`ORDER BY tier ASC, sort_order ASC`

### 4.3 缓存策略

- **进程内存缓存**：`Mutex<HashMap<Lang, (GiftListData, Instant)>>`，TTL 60s
- **缓存命中**：响应时间 <50ms
- **缓存失效**：T-10014 Admin CRUD 后调用 `invalidate_all()` 清除

### 4.4 架构设计

```
GiftHandler::list()
  ↓
GiftService::list_active(lang)
  ├─ 检查缓存：`cache.get(lang)` → 如果命中且未过期，直接返回
  └─ 缓存未命中 → PgGiftRepo::find_active(lang) → 构造响应 → 更新缓存
```

三层依赖注入：
- **PgGiftRepo** — 数据层，SQL 查询
- **GiftService** — 服务层，缓存管理
- **GiftHandler** — HTTP 层，请求路由

---

## 五、T-00020 SendGift 事务 + 广播

### 5.1 WS 信令设计

**客户端请求 `SendGift` (C→S)**
```json
{
  "type": "SendGift",
  "msg_id": "uuid",
  "payload": {
    "gift_id": "uuid",
    "receiver_id": "uuid",
    "count": 1
  }
}
```

**服务器响应 `SendGiftResult` (S→C)**
```json
{
  "type": "SendGiftResult",
  "msg_id": "uuid",
  "code": 0,
  "payload": {
    "gift_record_id": "uuid",
    "total_price": 520
  }
}
```

**房间广播 `GiftReceived` (S→房间)**
```json
{
  "type": "GiftReceived",
  "msg_id": "uuid",
  "payload": {
    "gift_record_id": "uuid",
    "sender": {
      "user_id": "uuid",
      "nickname": "Alice",
      "avatar": "https://..."
    },
    "receiver": {
      "user_id": "uuid",
      "nickname": "Bob",
      "avatar": null
    },
    "gift": {
      "id": "uuid",
      "code": "castle_01",
      "name": "قصر",
      "icon_url": "https://...",
      "animation_url": "https://...",
      "effect_level": 4
    },
    "count": 1,
    "total_price": 520
  },
  "timestamp": 1720000000000
}
```

**发送者推送 `BalanceUpdated` (S→C)**
```json
{
  "type": "BalanceUpdated",
  "msg_id": "uuid",
  "payload": {
    "diamond_balance": 4800,
    "delta": -520,
    "reason": "gift_send",
    "ref_id": "gift_record_id"
  },
  "timestamp": 1720000000000
}
```

### 5.2 事务核心流程

**6 步强事务** — 扣减 + 累加 + 流水 + 记录 + 榜单 + 广播

```
Client WS SendGift { gift_id, receiver_id, count, msg_id }
   ↓
ws/handler.rs :: handle_socket (路由分发)
   ↓
GiftSendService::send()
   │
   ├─ 1. 幂等检查：SELECT FROM gift_records WHERE sender_id=? AND msg_id=?
   │      └─ 命中 → 返回首次结果，不重复处理
   │
   ├─ 2. 数据查询：
   │      ├─ 校验发送者在房间
   │      ├─ 校验接收者在同房间且在麦上（读 RoomManager.get_room(room_id).mic_slots）
   │      ├─ 查礼物信息 SELECT price, code, name_ar, icon_url, animation_url, effect_level FROM gifts WHERE id=?
   │      └─ 计算 total_price = price * count
   │
   ├─ 3. BEGIN TX
   │      a) WalletService::apply_delta(sender, -total, "gift_send", ref_id=pending)
   │         └─ SELECT balance FOR UPDATE → 余额不足 → 回滚 → INSUFFICIENT_BALANCE (40290)
   │
   │      b) UPDATE users SET charm_balance += total WHERE id=receiver
   │
   │      c) INSERT gift_records(sender_id, receiver_id, room_id, gift_id, count, total_price, msg_id)
   │         └─ 返回 gift_record.id
   │
   │      d) UPDATE wallet_transactions SET ref_id=gift_record.id WHERE id=wallet_txn.id
   │      COMMIT
   │
   ├─ 4. Redis 榜单更新（事务提交后，非关键路径）
   │      ├─ ZINCRBY ranking:charm:day:{YYYY-MM-DD} total receiver_id
   │      ├─ ZINCRBY ranking:charm:week:{YYYY-WW} total receiver_id
   │      ├─ ZINCRBY ranking:wealth:day:{YYYY-MM-DD} total sender_id
   │      └─ ZINCRBY ranking:wealth:week:{YYYY-WW} total sender_id
   │
   ├─ 5. 房间广播 GiftReceived（给 registry.get_connections_in_room(room_id)）
   │
   ├─ 6. 发送者单播 BalanceUpdated（由 WalletService.apply_delta 事务提交后触发）
   │
   └─ 响应 SendGiftResult { code: 0, gift_record_id, total_price }
```

### 5.3 幂等设计

**两层防护**：

1. **业务层幂等**（相同 msg_id 第二次到达）
   - 先 `SELECT FROM gift_records WHERE (sender_id, msg_id) = (?, ?)` 检查
   - 命中 → 返回首次结果，**不重新扣款、不重新广播**

2. **数据库防护**（并发同时到达）
   - `UNIQUE (sender_id, msg_id)` 约束，第二条 INSERT 失败
   - 捕获约束异常后，重新 SELECT 返回首次结果

### 5.4 并发超扣防护

- `WalletService::apply_delta` 使用 `SELECT FOR UPDATE` 行锁
- 同一发送者最多 20 QPS 并发，无超扣、无脏数据
- SG10 验收标准：20 并发无超扣（测试覆盖）

### 5.5 错误码

| code | 常量名 | 含义 | 处理 |
|------|--------|------|------|
| `40001` | INVALID_COUNT | count 为 0 或超过 9999 | 客户端参数校验 |
| `40002` | MISSING_PARAMS | 参数缺失或格式非法 | 请求解析失败 |
| `40290` | INSUFFICIENT_BALANCE | 发送者钻石余额不足 | 事务回滚，余额不变 |
| `40400` | SENDER_NOT_IN_ROOM | 发送者不在指定房间 | 返回错误，不修改数据 |
| `40402` | GIFT_NOT_AVAILABLE | 礼物不存在或已下架 | 返回错误，不修改数据 |
| `40403` | RECEIVER_UNAVAILABLE | 接收者不在房间或不在麦上 | 返回错误，不修改数据 |

### 5.6 核心代码片段

**幂等检查**
```rust
// 先查是否已处理（业务层幂等）
if let Some(record) = repo.find_by_sender_and_msg_id(sender_id, &msg_id).await? {
    return Ok(SendGiftResult {
        code: 0,
        gift_record_id: record.id,
        total_price: record.total_price,
    });  // 不重发广播
}
```

**事务执行**
```rust
let mut txn = db.begin().await?;

// 1. 扣发送者余额（SELECT FOR UPDATE）
wallet_service.apply_delta(
    &mut txn,
    sender_id,
    -total_price,
    "gift_send",
    None,  // ref_id 先设为 None
).await?;

// 2. 加接收者魅力值
sqlx::query("UPDATE users SET charm_balance = charm_balance + ? WHERE id = ?")
    .bind(total_price)
    .bind(receiver_id)
    .execute(&mut *txn)
    .await?;

// 3. 写礼物记录
let gift_record = repo.insert_gift_record(&mut txn, ...).await?;

// 4. 更新流水 ref_id
wallet_service.update_wallet_txn_ref_id(&mut txn, ...).await?;

txn.commit().await?;
```

---

## 六、T-00020 实现状态与测试覆盖

### 6.1 验收标准（SG01~SG12）

| ID | 验收内容 | 状态 |
|----|---------|------|
| SG01 | 发送者余额 -total、接收者 charm_balance +total、gift_records +1、wallet_transactions +1 | ✅ |
| SG02 | 房间所有成员收到 GiftReceived | ✅ |
| SG03 | 发送者单独收到 BalanceUpdated { delta: -total } | ✅ |
| SG04 | Redis `ZSCORE ranking:charm:day:...` 更新为 total | ✅ |
| SG05 | 余额不足整体回滚，返回 INSUFFICIENT_BALANCE | ✅ |
| SG06 | 幂等：相同 (sender, msg_id) 二次发送，不扣款、不广播 | ✅ |
| SG07 | 接收者离开麦位返回 RECEIVER_UNAVAILABLE | ✅ |
| SG08 | gift 被下架返回 40402 | ✅ |
| SG09 | count=0 / count=10000 返回 40001 | ✅ |
| SG10 | 并发 20 QPS，无超扣、事务隔离 | ✅ |
| SG11 | 发送者不在房间返回 40400 | ✅ |
| SG12 | 事务中途任一步失败，余额不变、榜单不变、无广播 | ✅ |

### 6.2 代码审查修复（Round 1 → Round 2）

所有 6 个 Review Issue 已修复：

| Issue | 修复内容 | 状态 |
|-------|---------|------|
| **[C-1]** ranking.rs charm_day 双重计数 | 删除 `zadd`，改为纯 `zincr`；SG04 精确断言 | ✅ |
| **[H-1]** GiftReceived 缺失字段 | 补全 sender/receiver nickname+avatar，gift code/name/icon_url/animation_url/effect_level | ✅ |
| **[H-2]** Idempotent 死代码 | 删除 enum 变体和 handler match 臂 | ✅ |
| **[H-3]** protocol.md 错误码草稿值 | 更新为实现值（40001/40002/40290/40400/40402/40403） | ✅ |
| **[M-1]** try_send 静默丢弃 | 改为 `send().await`，有背压 | ✅ |
| **[L-1]** SG08 测试污染 | 用专用测试礼物隔离数据 | ✅ |

**Review 状态**：✅ Round 2 通过，批准合并

---

## 七、T-00021 榜单 API（设计中）

### 7.1 功能设计

```
GET /api/v1/ranking?type=charm|wealth&period=day|week&limit=50
```

- **返回值**：Top 50 + 当前用户排名 + Top 3 金银铜标识
- **查询延迟**：<100ms（读 Redis ZSet）
- **数据源**：Redis ZSet（由 T-00020 SendGift 更新）
- **定时任务**：
  - 每日 00:00 Riyadh 切换日榜 key（`ranking:charm:day:YYYY-MM-DD`）
  - 每周六切换周榜 key（`ranking:charm:week:YYYY-WW`）
- **归档策略**：旧榜迁移到 `ranking_archive` 表

### 7.2 Redis 键设计

| 键 | 含义 | TTL |
|----|------|-----|
| `ranking:charm:day:{YYYY-MM-DD}` | 日魅力榜（接收礼物累计） | 48h |
| `ranking:charm:week:{YYYY-WW}` | 周魅力榜 | 10d |
| `ranking:wealth:day:{YYYY-MM-DD}` | 日财富榜（发送礼物累计） | 48h |
| `ranking:wealth:week:{YYYY-WW}` | 周财富榜 | 10d |

### 7.3 实现模块

```
src/modules/gift/ranking.rs
├── increment_zscore()       # ZINCRBY 更新分数
├── get_top_users()          # 返回 Top N
├── get_user_rank()          # 查询当前用户排名
└── rotate_ranking_keys()    # 定时更新 key（每日 00:00 / 每周六）
```

---

## 八、性能指标与优化

| 指标 | 目标 | 实现 |
|------|------|------|
| 礼物列表响应 | <50ms | 进程内存缓存，TTL 60s |
| 发送礼物延迟 | <500ms | 事务提交后异步 Redis + WS 广播 |
| 并发送礼物 QPS | 20/s 无超扣 | SELECT FOR UPDATE 行锁 + UNIQUE 约束 |
| 榜单查询延迟 | <100ms | Redis ZSet 直读 |
| 缓存命中率 | >95% | 60s TTL 足够稳定 |

---

## 九、文档链接与关联

### 9.1 协议文档
- [WebSocket 信令设计](../protocol/websocket_signals.md) §6.4
  - SendGift 信令定义与错误码
  - GiftReceived 广播结构
  - BalanceUpdated 推送

### 9.2 TDS 文档
- [T-00019 礼物配置表 + 列表 API](../tds/server/T-00019.md)
- [T-00020 SendGift 事务 + 广播](../tds/server/T-00020.md)
- [T-00021 榜单 API](../tds/server/T-00021.md)（设计中）

### 9.3 产品文档
- [Phase 1 虚拟礼物与钱包闭环 MVP](../product/phase1_gift_economy.md) — E-07 Epic 方向总纲

### 9.4 任务看板
- [doc/tasks/index.md](../../tasks/index.md) — T-00019 ✅ / T-00020 ✅ / T-00021 开发中

---

## 十、遗留与下一步

### 10.1 已知限制

1. **缓存方案**（T-00019）
   - 当前：进程内存缓存
   - 设计：Redis（TDS）
   - 影响：单实例无问题，多实例部署时需切换 Redis

2. **榜单定时任务**（T-00021）
   - 未实现日/周切换 task
   - 影响：当前榜单 key 需手动管理

3. **幂等存储期限**（T-00020）
   - 当前：DB UNIQUE 约束永久存储
   - 可优化：7 天后可清理历史 gift_records（成本 vs 防重复）

### 10.2 下一步计划

| Task | 描述 | 依赖 |
|------|------|------|
| T-00021 | 榜单 API + 定时任务 | T-00020 ✅ |
| T-10013 | Admin 礼物管理（CRUD） | T-00019 ✅ |
| T-10014 | Admin 榜单查看 | T-00021 待实现 |
| T-30030 | Android SendGift UI + 幂等 | T-00020 ✅ |
| T-20012 | Web 礼物商城页 | T-00019 ✅ |

---

## 十一、常见问题

**Q: 为什么 GiftReceived 包含完整的 sender/receiver/gift 信息而不是仅 ID？**  
A: 客户端渲染礼物动画需要展示用户头像+昵称、礼物图标+效果等，避免重复查询。

**Q: 余额不足时为什么仍要提交事务？**  
A: 检查是在 SELECT FOR UPDATE 后，事务自动 rollback。流程上不"提交"成功事务，是"失败回滚"。

**Q: Redis 榜单失败是否影响发送礼物结果？**  
A: 不影响。Redis 非关键路径，连接失败仅记 warn 日志，用户余额和魅力值已正确更新在 DB。

---

**文档编写**：Copilot DoD Agent  
**最后更新**：2025-06-27  
**Review 状态**：✅ T-00020 Round 2 通过
