# Nobility API（E-09 贵族体系）

> **版本**: v1.0 · 2026-05-10
> **关联产品文档**: [phase1_nobility.md](../product/phase1_nobility.md)（**§3.5 贵族特权细则为字段值唯一来源**）
> **关联模块**: 模块 11 - 贵族体系 (E-09)
>
> ⚠️ **字段冻结**：本文件是 E-09 字段级唯一事实源。

---

## 10.1 概览

E-09 协议组成：
1. **HTTP REST**（客户端 → App Server）：tier 列表、我的贵族、钻石购买、续费、续费提醒确认
2. **HTTP REST**（Admin Server）：tier CRUD、手动赠送/撤销、查询导出
3. **WS S→C 单播**：`NobleRenewSuccess` / `NobleRenewFailed` / `NobleExpired` / `NobleChanged`
4. **WS S→Room 广播**：`NobleEntered`（房间内进场特效；`UserJoined.noble` 字段补充扩展）
5. **WS S→Global 广播**：`NobleEntranceGlobal`（公爵 LV5+ 全服跑马灯）— 通过 Redis Pub/Sub channel `noble:global` 多实例分发

## 10.2 数据模型

### 10.2.1 `noble_tiers` 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `tier_id` | `VARCHAR(32)` | PRIMARY KEY | 例：`knight` / `baron` / `viscount` / `earl` / `duke` / `king` |
| `name_en` | `VARCHAR(64)` | NOT NULL | 英文名 |
| `name_ar` | `VARCHAR(64)` | NOT NULL | 阿拉伯文名 |
| `level` | `SMALLINT` | NOT NULL UNIQUE CHECK 1..6 | 1~6 |
| `monthly_diamonds` | `BIGINT` | NOT NULL CHECK > 0 | 月费钻石（产品 §1.3 表）|
| `monthly_usd` | `NUMERIC(10,2)` | NOT NULL | 月费美元（真金通道 SKU 价）|
| `usd_sku_id` | `VARCHAR(64)` | NULL FK→payment_skus | 真金 SKU（仅 LV3+ 提供）|
| `privileges` | `JSONB` | NOT NULL | 见 §10.2.3 |
| `icon_url` | `TEXT` | NOT NULL | 徽章图标 |
| `frame_url` | `TEXT` | NOT NULL | 头像框 |
| `entrance_animation_url` | `TEXT` | NULL | 进场 Lottie（LV3+ 必填，LV1/2 NULL）|
| `bgm_url` | `TEXT` | NULL | 进场 BGM（LV2+ 必填）|
| `badge_color` | `VARCHAR(16)` | NOT NULL | 主色调 hex（产品 §3.5.1）|
| `bubble_style_id` | `VARCHAR(32)` | NOT NULL | 客户端气泡样式键 |
| `is_active` | `BOOLEAN` | NOT NULL DEFAULT TRUE | 上下架（软删保留历史持有者）|
| `created_at` / `updated_at` | `TIMESTAMPTZ` | NOT NULL | - |

### 10.2.2 `user_nobles` 表（一人最多一个有效贵族）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `user_id` | `UUID` | PRIMARY KEY FK→users | **唯一约束确保单贵族**|
| `tier_id` | `VARCHAR(32)` | NOT NULL FK→noble_tiers | 当前等级 |
| `start_at` | `TIMESTAMPTZ` | NOT NULL | 首次开通时间 |
| `current_period_start` | `TIMESTAMPTZ` | NOT NULL | 当前续费周期起点 |
| `expire_at` | `TIMESTAMPTZ` | NOT NULL | 当前周期到期 |
| `auto_renew` | `BOOLEAN` | NOT NULL DEFAULT TRUE | 自动续费开关 |
| `renew_channel` | `VARCHAR(16)` | NOT NULL | `diamonds` \| `usd` \| `admin_grant` |
| `failed_renew_count` | `INT` | NOT NULL DEFAULT 0 | 连续续费失败次数（≥3 关闭 auto_renew）|
| `total_paid_diamonds` | `BIGINT` | NOT NULL DEFAULT 0 | 累计扣钻 |
| `total_paid_usd_micros` | `BIGINT` | NOT NULL DEFAULT 0 | 累计真金 micros |
| `last_changed_msg_id` | `VARCHAR(64)` | NULL | 最近一次 NobleChanged 广播的 envelope.msg_id（用于客户端去重）|
| `created_at` / `updated_at` | `TIMESTAMPTZ` | NOT NULL | - |

**索引**：
```sql
CREATE INDEX idx_user_nobles_expire ON user_nobles (expire_at);
CREATE INDEX idx_user_nobles_auto_renew ON user_nobles (auto_renew, expire_at)
  WHERE auto_renew = TRUE;
```

### 10.2.3 `noble_tiers.privileges` JSONB Schema

```jsonc
{
  "badge": {
    "color": "#DC2626",
    "shape": "crown_large",
    "animated": true       // LV6 国王 true
  },
  "entry_effect": {
    "duration_ms": 8000,
    "scope": "fullscreen",  // marquee | half | fullscreen
    "marquee_color": "red_gold",
    "user_can_disable": false
  },
  "chat_bubble": {
    "style_id": "king",
    "gradient": ["#FCA5A5", "#F59E0B"],
    "border_color": "#F59E0B",
    "username_color": "#991B1B"
  },
  "audience_pin": {
    "scope": "global",      // none | own_room | own_lobby | global
    "rank_offset": 1
  },
  "invisibility": {
    "scope": "all",         // none | mic_only | mic_and_audience | all
    "always_visible_to": ["admin"]
  },
  "bypass_password": {
    "enabled": true,
    "respect_room_owner_switch": true   // 房主关掉时仍需密码
  },
  "mic_priority": {
    "weight": 10.0
  },
  "gift_discount": {
    "percent": 15           // 0..100；服务端结算 = floor(price × (100-percent) / 100)
  },
  "global_broadcast": {
    "enabled": true,
    "daily_limit": 1
  },
  "vip_support": {
    "sla_minutes": 5
  },
  "monthly_stipend": {
    "percent": 20,           // 0..100
    "pay_immediately": true
  },
  "expiry": {
    "warn_days_before": 3,
    "grace_days": 7,
    "history_days": 30
  }
}
```

服务端必须以 [`schemas/http/NobilityPrivileges.schema.json`](schemas/http/NobilityPrivileges.schema.json)（待新建）做 JSON Schema Draft 2020-12 校验。

### 10.2.4 6 档种子（产品 §3.5 抽取）

| tier_id | level | monthly_diamonds | monthly_usd | discount | mic_weight | stipend% | invisibility | bypass_pw | global_bc |
|---------|-------|-----------------:|------------:|---------:|-----------:|---------:|--------------|-----------|-----------|
| knight | 1 | 3000 | 9.99 | 0 | 1.0 | 5 | none | no | no |
| baron | 2 | 10000 | 29.99 | 2 | 1.0 | 8 | none | no | no |
| viscount | 3 | 30000 | 99.99 | 5 | 1.0 | 10 | none | no | no |
| earl | 4 | 100000 | 299.99 | 8 | 1.5 | 12 | mic_only | no | no |
| duke | 5 | 300000 | 999.99 | 10 | 3.0 | 15 | mic_and_audience | yes | yes(1/d) |
| king | 6 | 1000000 | 3999.99 | 15 | 10.0 | 20 | all | yes | yes(1/d) |

### 10.2.5 `noble_history` 表（审计）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `BIGSERIAL` PK | - |
| `user_id` | `UUID` | - |
| `event` | `VARCHAR(32)` | `purchase` \| `renew_success` \| `renew_failed` \| `upgrade` \| `downgrade_attempt` \| `expire` \| `admin_grant` \| `admin_revoke` |
| `from_tier` | `VARCHAR(32)` | 可空 |
| `to_tier` | `VARCHAR(32)` | 可空 |
| `payload` | `JSONB` | 详细字段 |
| `actor` | `VARCHAR(64)` | `user:<uuid>` \| `system:cron` \| `admin:<uuid>` |
| `created_at` | `TIMESTAMPTZ` | - |

---

## 10.3 HTTP REST：客户端 → App Server

### 10.3.1 `GET /api/v1/nobles/tiers`

获取所有上架 tier（无需鉴权；带本地化）。

**Headers**：`Accept-Language: ar-SA` / `en-US`（默认 en-US）

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "tiers": [
      {
        "tier_id": "duke",
        "name": "公爵",
        "level": 5,
        "monthly_diamonds": 300000,
        "monthly_usd": "999.99",
        "usd_sku_id": "noble_duke_30d",
        "icon_url": "https://cdn.../duke_icon.svg",
        "frame_url": "https://cdn.../duke_frame.png",
        "entrance_animation_url": "https://cdn.../duke_entry.json",
        "bgm_url": "https://cdn.../duke_bgm.mp3",
        "badge_color": "#06B6D4",
        "bubble_style_id": "duke",
        "privileges": { /* §10.2.3 完整 JSON */ }
      }
    ]
  }
}
```

### 10.3.2 `GET /api/v1/nobles/me`

获取当前用户贵族信息（鉴权）。

**Response 200（持有）**：
```jsonc
{
  "code": 0,
  "data": {
    "tier_id": "duke",
    "level": 5,
    "start_at": "2026-04-10T00:00:00Z",
    "current_period_start": "2026-05-10T00:00:00Z",
    "expire_at": "2026-06-09T00:00:00Z",
    "auto_renew": true,
    "renew_channel": "diamonds",
    "days_remaining": 30,
    "in_grace_period": false,
    "tier": { /* 完整 tier 对象 */ }
  }
}
```

**Response 200（未持有）**：
```jsonc
{ "code": 0, "data": { "tier_id": null } }
```

### 10.3.3 `POST /api/v1/nobles/purchase`

钻石通道购买/续费/升级（鉴权）。

**Body**：
```jsonc
{
  "tier_id": "duke",
  "msg_id": "client-uuid-v4",          // 幂等键（24h 内复用）
  "auto_renew": true,
  "duration_days": 30                  // 30 | 90 | 365；默认 30
}
```

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "user_noble": { /* 同 10.3.2 持有响应 */ },
    "diamonds_charged": 300000,
    "balance_after": 1000000,
    "operation": "purchase",            // purchase | renew | upgrade
    "upgrade_proration": null           // 升级时含 { from_tier, refund_diamonds, charge_diamonds }
  }
}
```

**升级补差公式**（产品 §1.3）：
```
remaining_days = (expire_at - now) / 1d
refund_diamonds = floor(old_tier.monthly_diamonds × remaining_days / 30)
charge_diamonds = max(0, new_tier.monthly_diamonds × duration_days/30 - refund_diamonds)
```

**错误码**：见 §10.6（重点 40911 DOWNGRADE_NOT_ALLOWED / 40912 INSUFFICIENT_BALANCE / 40913 SAME_TIER_OVERLAP）。

### 10.3.4 `POST /api/v1/nobles/usd_purchase_intent`

真金通道下单意图（鉴权）；返回 E-08 订单与对应 SKU。

**Body**：
```jsonc
{
  "tier_id": "duke",
  "duration_days": 30,
  "msg_id": "client-uuid-v4"
}
```

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "payment_order_id": "...",      // 转交客户端走 E-08 verify 链路
    "sku_id": "noble_duke_30d",
    "diamonds_to_credit": 0,        // 真金贵族 SKU 不发钻石；ack 后服务端直接 upsert user_nobles
    "tier_id": "duke",
    "duration_days": 30
  }
}
```

> **服务端实现要点**：当 E-08 订单 `state=ACKED` 且关联 SKU `tag='noble_pack'` 时，强事务额外执行 `noble_grant(user_id, tier_id, duration_days, channel='usd')`，并广播 `NobleChanged`。

### 10.3.5 `PATCH /api/v1/nobles/me/auto_renew`

切换自动续费开关。

**Body**：`{ "enabled": false }`

**Response 200**：`{ "code": 0, "data": { "auto_renew": false } }`

### 10.3.6 `POST /api/v1/nobles/me/renew_reminder/ack`

客户端关闭"续费提醒"弹窗后回写（用于 24h 内不再弹）。

**Body**：`{ "kind": "renew_failed" | "expire_warn" | "expired", "event_id": "..." }`

**Response 200**：`{ "code": 0 }`

---

## 10.4 WS 信令（C→S 无；S→C / S→Room）

### 10.4.1 `NobleChanged`（S→C 单播 + S→Room 广播）

> **路由**：用户购买/续费/升级/降档/被赠送/被撤销 → S→C 单播给本人；同时若用户当前在房间内 → S→Room 广播（更新观众列表/麦位徽章）。

**Schema**：[schemas/ws/NobleChanged.schema.json](schemas/ws/NobleChanged.schema.json)（新增）

```jsonc
{
  "type": "NobleChanged",
  "msg_id": "...",
  "ts": 1746788688000,
  "payload": {
    "user_id": "...",
    "from_tier_id": "earl",      // 可空（首次购买）
    "to_tier_id": "duke",        // 可空（撤销 / 过期）
    "expire_at": "2026-06-09T00:00:00Z",
    "reason": "purchase"          // purchase | renew | upgrade | admin_grant | admin_revoke | expire
  }
}
```

### 10.4.2 `NobleRenewSuccess`（S→C 单播）

```jsonc
{
  "type": "NobleRenewSuccess",
  "payload": {
    "tier_id": "duke",
    "diamonds_charged": 300000,
    "new_expire_at": "2026-07-09T00:00:00Z",
    "stipend_credited": 60000
  }
}
```

### 10.4.3 `NobleRenewFailed`（S→C 单播）

```jsonc
{
  "type": "NobleRenewFailed",
  "payload": {
    "tier_id": "duke",
    "reason": "INSUFFICIENT_BALANCE",   // INSUFFICIENT_BALANCE | TIER_INACTIVE | UNKNOWN
    "failed_count": 2,                   // 累计失败
    "auto_renew_disabled_after": 3,
    "next_retry_at": "2026-05-11T01:00:00Z"
  }
}
```

### 10.4.4 `NobleExpired`（S→C 单播）

```jsonc
{
  "type": "NobleExpired",
  "payload": {
    "tier_id": "duke",
    "expired_at": "2026-06-09T00:00:00Z",
    "in_grace_period": true,
    "grace_until": "2026-06-16T00:00:00Z"
  }
}
```

### 10.4.5 `NobleEntered`（S→Room 广播）

> **触发**：贵族用户 JoinRoom 成功后服务端按 `noble_tiers.privileges.entry_effect` 决定广播范围。同时 `UserJoined.noble` 字段也下发（双信号互补；`UserJoined` 用于列表渲染，`NobleEntered` 用于特效播放队列）。

**Schema**：[schemas/ws/NobleEntered.schema.json](schemas/ws/NobleEntered.schema.json)（新增）

```jsonc
{
  "type": "NobleEntered",
  "payload": {
    "user_id": "...",
    "nickname": "阿里",
    "avatar_url": "...",
    "tier_id": "duke",
    "level": 5,
    "entrance_animation_url": "https://cdn.../duke_entry.json",
    "bgm_url": "https://cdn.../duke_bgm.mp3",
    "duration_ms": 6000,
    "scope": "fullscreen",                  // marquee | half | fullscreen
    "marquee_text_en": "Duke Ali has arrived",
    "marquee_text_ar": "وصل الدوق علي",
    "frame_url": "https://cdn.../duke_frame.png"
  }
}
```

**隐身规则**（产品 §3.5.5）：服务端按等级 `privileges.invisibility.scope` 过滤接收者：
- `none` / `mic_only` → 全房间收
- `mic_and_audience` → 仅房主/Admin/麦上用户收
- `all` (king) → 仅 Admin 收（默认；可被房主白名单覆盖）

### 10.4.6 `NobleEntranceGlobal`（S→Global 全服跑马灯）

> **触发**：仅 LV5 公爵 / LV6 国王 登录或首次购买时；通过 Redis Pub/Sub `noble:global` 多实例分发。

```jsonc
{
  "type": "NobleEntranceGlobal",
  "payload": {
    "user_id": "...",
    "nickname": "阿里",
    "tier_id": "king",
    "kind": "login_daily",             // login_daily | first_purchase
    "marquee_text_en": "...",
    "marquee_text_ar": "..."
  }
}
```

频控：`noble_global_broadcast_log(user_id, kind, broadcast_date)` 唯一约束 → 每日 1 次。

### 10.4.7 `UserJoined.noble` 字段扩展（已有信令的字段补充）

复用 `UserJoined`（[websocket_signals.md §6.7.1](websocket_signals.md#671-userjoinedsroom)），payload 新增可选字段：

```jsonc
{
  "type": "UserJoined",
  "payload": {
    "user_id": "...",
    "nickname": "...",
    "avatar_url": "...",
    "noble": {                       // 新增；用户无贵族时省略此字段
      "tier_id": "duke",
      "level": 5,
      "badge_color": "#06B6D4",
      "frame_url": "...",
      "expire_at": "2026-06-09T00:00:00Z"
    }
  }
}
```

**MemberSnapshot 同步扩展**：所有返回用户列表的接口（`JoinRoomResult.members`、观众席、麦位 list）项目内嵌 `noble` 字段，结构同上。

### 10.4.8 `BalanceUpdated.reason` 扩展

复用既有 `BalanceUpdated`（[§6.8.3](websocket_signals.md#683-balanceupdatedsc-单播)）。E-09 新增 `reason` 取值：
- `noble_purchase`
- `noble_renew`
- `noble_upgrade_proration`
- `noble_stipend`
- `noble_gift_discount_subsidy`
- `admin_noble_grant_rollback`

---

## 10.5 Admin REST（Admin Server）

| 路径 | 方法 | 权限 | 说明 |
|------|------|------|------|
| `/api/v1/admin/nobles/tiers` | GET / POST / PUT / DELETE | `noble.write` | tier CRUD（含 privileges JSON Schema 校验）|
| `/api/v1/admin/users/:id/noble/grant` | POST | `super_admin` | 手动赠送 `{ tier_id, days, reason }` |
| `/api/v1/admin/users/:id/noble/revoke` | POST | `super_admin` | 撤销 `{ reason }` |
| `/api/v1/admin/nobles/users` | GET | `noble.read` | 用户列表 + 筛选 + CSV |

详细 DTO 在 [admin_api.md](admin_api.md) 待 T-10030/31/32 追加章节。

---

## 10.6 错误码表（E-09）

| code | HTTP | message | 触发 |
|------|------|---------|-----|
| `40911` | 409 | DOWNGRADE_NOT_ALLOWED | 持有高档贵族尝试购买低档 |
| `40912` | 422 | INSUFFICIENT_BALANCE | 钻石不足（含升级补差）|
| `40913` | 409 | SAME_TIER_RENEWAL_OVERLAP | 同档续费但当前 `expire_at - now > 30d`（避免无限囤）|
| `40914` | 404 | TIER_INACTIVE | tier 已下架 |
| `40915` | 409 | NOBLE_PRIVILEGE_BLOCKED | 特权被禁用（如风控临时关闭某用户的全服广播）|
| `40916` | 409 | RENEW_REMINDER_ACK_INVALID | event_id 不存在或已确认 |
| `40917` | 422 | PRIVILEGES_SCHEMA_INVALID | Admin 提交的 privileges JSON 不符合 schema |

---

## 10.7 特权钩子绑定（服务端 enforcement map，引用产品 §3.5.13）

| 特权 | 服务端模块 | 引用入口 |
|------|----------|---------|
| 徽章/气泡/资料框 | `RenderContextBuilder` | 所有返回用户对象的 API/WS 响应在序列化前调用 |
| 进场特效 | `RoomService::on_join` | 见 §10.4.5 / §10.4.7 |
| 观众席置顶 | `AudienceListService::list_with_priority` | 复用 §3.3 GET /rooms/:id/audience，按 (noble_level desc, joined_at asc) |
| 隐身 | `PresenceService::list_visible_users` | 见 §10.4.5 隐身规则 |
| 免密 | `JoinRoomService::check_password_or_bypass` | JoinRoom 处理函数中前置判断 |
| 优先抢麦 | `MicQueueService::try_acquire_with_priority` | TakeMic Lua 脚本 / Rust 端 softmax 抽签 |
| 礼物折扣 | `GiftService::send` | SendGift 处理函数；`gift_transactions.discount_pct` 字段记账 |
| 全服广播 | `LoginService::on_success` → `noble:global` Pub/Sub | 见 §10.4.6 |
| 月津贴 | `NobilityRenewalService::grant_stipend` | renew/purchase 事务尾部 |
| 过期降级 | `NobilityExpiryCron`（每小时）| 状态机推进 + WS 推送 |

---

## 10.8 关联文档

- [phase1_nobility.md](../product/phase1_nobility.md)（产品方向 + §3.5 特权细则）
- [模块11-贵族体系 (E-09).md](../tasks/模块11-贵族体系%20(E-09).md)（Tasks）
- [websocket_signals.md](websocket_signals.md)（信令骨架）
- [payment_api.md](payment_api.md)（真金通道 §10.3.4 复用）
- [conventions.md §1.4](conventions.md)（错误码模块11）
