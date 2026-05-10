# Payment API（E-08 Google Play 真支付）

> **版本**: v1.0 · 2026-05-10
> **关联产品文档**: [phase1_payment_billing.md](../product/phase1_payment_billing.md)
> **关联模块**: 模块 10 - Google Play 真支付 (E-08)
> **Google 官方权威（字段值唯一来源）**:
> - https://developer.android.com/google/play/billing/integrate
> - https://developer.android.com/google/play/billing/security
> - https://developer.android.com/google/play/billing/rtdn-reference
> - https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.products
>
> ⚠️ **字段冻结**：本文件是 E-08 字段级唯一事实源。所有 TDS / 实现 / 测试断言必须**逐字段**引用此处定义。

---

## 9.1 概览

E-08 包含三类协议：
1. **HTTP REST**（客户端 → App Server / Admin Server）：SKU 查询、订单生命周期、对账查询、Admin 报表、Dev Mock 通道
2. **WS S→C 单播**：余额变更（复用既有 `BalanceUpdated`）
3. **HTTP Webhook**（Google Cloud Pub/Sub Push → App Server）：RTDN 异步事件

## 9.2 数据模型

### 9.2.1 `payment_skus` 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `sku_id` | `VARCHAR(64)` | PRIMARY KEY | 与 Google Play Console 的 `productId` **字符串一致** |
| `provider` | `payment_provider` ENUM | NOT NULL | 取值：`google_play` \| `apple_iap`（预留）\| `mock`（仅 dev/staging）|
| `diamonds` | `BIGINT` | NOT NULL CHECK > 0 | 充值钻石数 |
| `display_price_usd` | `NUMERIC(10,2)` | NOT NULL | 展示用美元价（参考价；实际扣款以 Google 返回 `priceAmountMicros` 为准）|
| `display_price_local` | `NUMERIC(12,2)` | NULL | 展示用本地货币（可空，Android 端用 BillingClient 返回值覆盖）|
| `display_currency` | `VARCHAR(3)` | NULL | ISO 4217 |
| `is_active` | `BOOLEAN` | NOT NULL DEFAULT TRUE | 上下架（软删保留历史订单）|
| `sort_order` | `INT` | NOT NULL DEFAULT 0 | 客户端展示顺序 |
| `tag` | `VARCHAR(32)` | NULL | 例：`hot` / `best_value` / `noble_pack` |
| `created_at` / `updated_at` | `TIMESTAMPTZ` | NOT NULL | - |

种子（5 档钻石包）：

| sku_id | diamonds | display_price_usd | tag |
|--------|---------:|------------------:|-----|
| `diamond_60` | 60 | 0.99 | - |
| `diamond_300` | 300 | 4.99 | - |
| `diamond_600` | 600 | 9.99 | hot |
| `diamond_1980` | 1980 | 29.99 | best_value |
| `diamond_6480` | 6480 | 99.99 | - |

### 9.2.2 `payment_orders` 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `order_id` | `UUID` | PRIMARY KEY | 服务端预生成；同时作为 `obfuscatedAccountId` 上传 Google |
| `user_id` | `UUID` | NOT NULL FK→users | 下单用户 |
| `sku_id` | `VARCHAR(64)` | NOT NULL FK→payment_skus | - |
| `provider` | `payment_provider` | NOT NULL | - |
| `purchase_token` | `TEXT` | NULL UNIQUE WHERE NOT NULL | **真主键语义**（Google 安全文档要求；promo code 场景无 orderId）|
| `provider_order_id` | `VARCHAR(64)` | NULL | Google `orderId`，仅审计；可空 |
| `amount_micros` | `BIGINT` | NULL | Google 返回 `priceAmountMicros`（`amount × 1_000_000`）|
| `currency` | `VARCHAR(3)` | NULL | Google 返回 `priceCurrencyCode` |
| `country_code` | `VARCHAR(2)` | NULL | Google 返回 `countryCode` |
| `state` | `payment_order_state` ENUM | NOT NULL DEFAULT 'PENDING' | 见 §9.2.3 |
| `state_history` | `JSONB` | NOT NULL DEFAULT `[]` | 状态推进时间线，元素 `{state, ts, source}` |
| `risk_flags` | `TEXT[]` | NOT NULL DEFAULT `{}` | 风控标签 |
| `idempotency_key` | `VARCHAR(64)` | NULL | 客户端创建订单时 `Idempotency-Key` 头透传 |
| `dev_mock_outcome` | `VARCHAR(16)` | NULL | 仅 mock 通道使用：`success` \| `fail` \| `pending` |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | - |
| `verified_at` | `TIMESTAMPTZ` | NULL | - |
| `credited_at` | `TIMESTAMPTZ` | NULL | - |
| `acked_at` | `TIMESTAMPTZ` | NULL | - |
| `failed_at` | `TIMESTAMPTZ` | NULL | - |
| `failed_reason` | `VARCHAR(64)` | NULL | 错误码字符串 |

**索引**：
```sql
CREATE INDEX idx_orders_user_created ON payment_orders (user_id, created_at DESC);
CREATE UNIQUE INDEX uq_orders_provider_purchase_token
  ON payment_orders (provider, purchase_token) WHERE purchase_token IS NOT NULL;
CREATE INDEX idx_orders_state_pending
  ON payment_orders (state, created_at)
  WHERE state IN ('PENDING','VERIFYING','VERIFIED','CREDITED');
```

### 9.2.3 状态枚举 `payment_order_state`

| 值 | 含义 | 进入条件 | 是否终态 |
|----|------|---------|---------|
| `PENDING` | 服务端预创建 | `POST /payments/orders` 写入 | 否 |
| `VERIFYING` | 收到 token 待验签 | `POST /payments/google/verify` 入参合法 | 否 |
| `VERIFIED` | Google 验签通过 | `Purchases.products:get` 返回 `purchaseState=0` 且校验通过 | 否 |
| `CREDITED` | 钻石入账事务完成 | 强事务成功提交 | 否（待 ack） |
| `ACKED` | Google 已 acknowledge / consume | Google API 返回 200 | ✅ |
| `CANCELLED` | 用户在 Play 弹窗取消 | onPurchasesUpdated `BillingResponseCode.USER_CANCELED` 上报 | ✅ |
| `FAILED` | 验签失败 / 风控拦截 / token 重放 | 见 §9.5 错误码表 | ✅ |
| `REFUNDED` | 收到 RTDN VoidedPurchaseNotification | RTDN 处理事务完成 | ✅ |
| `PENDING_GOOGLE` | Google 返回 `purchaseState=2 (PENDING)` | 慢速测试卡 / 现金支付 | 否 |

### 9.2.4 `wallet_transactions` 字段扩展

复用既有 `wallet_transactions` 表（T-00017）。本 Epic 新增 `source` 取值：
| `source` | 含义 |
|---------|------|
| `recharge_google_play` | Google Play 充值（生产 + License Tester）|
| `dev_mock` | Dev/Staging Mock 通道（不计入财务报表）|
| `noble_stipend` | 贵族月津贴（E-09 复用）|
| `gift_discount_subsidy` | 贵族礼物折扣由平台补贴部分（E-09）|
| `refund_google_play` | RTDN 退款回扣 |
| `admin_recredit` | Admin 手动补单 |
| `admin_refund` | Admin 手动退款 |

### 9.2.5 `rtdn_processed` 表（幂等去重）

| 字段 | 类型 | 约束 |
|------|------|------|
| `message_id` | `VARCHAR(64)` | PRIMARY KEY（Pub/Sub `message.messageId`）|
| `event_time_millis` | `BIGINT` | NOT NULL |
| `notification_kind` | `VARCHAR(32)` | NOT NULL（`oneTimeProductNotification` \| `voidedPurchaseNotification` \| `subscriptionNotification` \| `testNotification`）|
| `purchase_token` | `TEXT` | NULL |
| `processed_at` | `TIMESTAMPTZ` | NOT NULL DEFAULT now() |
| `outcome` | `VARCHAR(32)` | NOT NULL（`applied` \| `ignored_duplicate` \| `ignored_unknown_token` \| `error`）|

---

## 9.3 HTTP REST：客户端 → App Server

### 9.3.1 `GET /api/v1/payments/skus`

获取上架 SKU 列表（公开，无需鉴权）。

**Query**：`provider=google_play`（默认 google_play）

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "skus": [
      {
        "sku_id": "diamond_600",
        "provider": "google_play",
        "diamonds": 600,
        "display_price_usd": "9.99",
        "display_price_local": null,
        "display_currency": null,
        "tag": "hot",
        "sort_order": 30
      }
    ]
  }
}
```

**字段说明**：
- `display_price_local` / `display_currency` 后端可返回 null；客户端**最终**应使用 `BillingClient.queryProductDetailsAsync` 返回的 `formattedPrice` 渲染（按 Google 文档要求展示本地化价格）。

### 9.3.2 `POST /api/v1/payments/orders`

创建订单（鉴权：用户 JWT）。

**Headers**：`Idempotency-Key: <UUIDv4>`（建议；同一 key 24h 内复用同一订单）

**Body**：
```jsonc
{
  "sku_id": "diamond_600",
  "provider": "google_play",
  "client_session_id": "and-session-uuid"  // 可空，埋点透传
}
```

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "order_id": "0e3c...d4ab",   // UUID，必须作为 obfuscatedAccountId 传给 Google
    "sku": { /* 同 9.3.1 单条 */ },
    "expire_at": "2026-05-10T12:30:00Z"  // 30 分钟未支付订单服务端自动 CANCELLED
  }
}
```

**错误码**：见 §9.6。

### 9.3.3 `POST /api/v1/payments/google/verify`

提交 Google purchaseToken 进入验签链路（鉴权：用户 JWT）。

**Body**：
```jsonc
{
  "order_id": "0e3c...d4ab",
  "purchase_token": "oojklmnopqrstuvwx...",     // BillingClient.Purchase.purchaseToken
  "provider_order_id": "GPA.0001-xxxx"          // 可选；客户端 Purchase.orderId（可能为空）
}
```

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "order_id": "0e3c...d4ab",
    "state": "ACKED",          // 终态：ACKED / CREDITED（acknowledge 暂未完成）/ PENDING_GOOGLE / FAILED
    "diamonds_credited": 600,  // 实际入账钻石（仅 CREDITED/ACKED 有值）
    "balance_after": 1860,     // 实际入账后用户余额；其他状态为 null
    "next_action": "wait_rtdn" // 当 state=PENDING_GOOGLE：客户端只显示"等待 Google 确认"
  }
}
```

**幂等**：相同 `purchase_token` 重复调用安静返回首次结果。

**错误码**：见 §9.6（重点：40901 INVALID_PURCHASE / 40903 ORDER_RISK_BLOCKED / 40904 ORDER_NOT_FOUND）。

### 9.3.4 `GET /api/v1/orders/me`

查询自己的订单（鉴权）。

**Query**：`status=`（可空，多值逗号分隔）`cursor=` `limit=20`

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "orders": [
      {
        "order_id": "...",
        "sku_id": "diamond_600",
        "diamonds": 600,
        "amount_micros": 9990000,
        "currency": "USD",
        "state": "ACKED",
        "created_at": "...",
        "credited_at": "...",
        "acked_at": "..."
      }
    ],
    "next_cursor": "...",
    "has_more": true
  }
}
```

### 9.3.5 `GET /api/v1/orders/me/:orderId`

查询单个订单详情（鉴权 + 必须本人）。

**Response 200**：
```jsonc
{
  "code": 0,
  "data": {
    "order_id": "...",
    "sku_id": "diamond_600",
    "diamonds": 600,
    "amount_micros": 9990000,
    "currency": "USD",
    "country_code": "SA",
    "state": "ACKED",
    "state_history": [
      { "state": "PENDING", "ts": "...", "source": "client_create" },
      { "state": "VERIFYING", "ts": "...", "source": "client_verify" },
      { "state": "VERIFIED", "ts": "...", "source": "google_get" },
      { "state": "CREDITED", "ts": "...", "source": "tx_commit" },
      { "state": "ACKED", "ts": "...", "source": "google_ack" }
    ],
    "purchase_token_masked": "oojkl...XXXX",  // 前 5 + 后 4，中间脱敏
    "provider_order_id": "GPA.0001-xxxx"
  }
}
```

---

## 9.4 HTTP REST：客户端 → App Server（Dev/Staging Mock）

### 9.4.1 `POST /api/v1/_dev/mock_recharge`

> ⚠️ **环境限定**：仅当服务端 `payment.mock_enabled=true` AND `profile != production` 时注册路由。生产环境 panic on boot；运行期返回 404。

**Body**：
```jsonc
{
  "sku_id": "diamond_600",
  "force_outcome": "success",  // success | fail | pending
  "client_note": "qa-tag-xxx"  // 可选，写入 order.dev_mock_outcome
}
```

**Response 200（success / pending）**：
```jsonc
{
  "code": 0,
  "data": {
    "order_id": "...",
    "state": "ACKED",                   // success → ACKED；pending → PENDING_GOOGLE；fail → FAILED
    "diamonds_credited": 600,           // 仅 success
    "balance_after": 1860,
    "wallet_transaction_id": "..."      // wallet_transactions.source = 'dev_mock'
  }
}
```

**Response 4xx（fail）**：返回 `40901 MOCK_FORCED_FAIL` + `state=FAILED` 已写入。

**链路约束**：与生产链路共用同一 `BillingPort` 接口；mock 通道注入 `MockBillingPort` 实现，**不直接旁路**。

---

## 9.5 HTTP Webhook：Google RTDN

### 9.5.1 端点：`POST /webhook/google/rtdn`

**鉴权**：Cloud Pub/Sub Push 模式 + Service Account OIDC 令牌验证（验证 `Authorization: Bearer <jwt>` 中 `aud` 与本服务订阅配置一致）。

**Body**（Pub/Sub 包络，**字段名禁止改造**）：
```jsonc
{
  "message": {
    "messageId": "136969346945",
    "publishTime": "2026-05-09T10:24:48.690Z",
    "data": "<base64 of DeveloperNotification JSON>",
    "attributes": { /* 可空 */ }
  },
  "subscription": "projects/voiceroom/subscriptions/rtdn-sub-prod"
}
```

base64 解出后的 `DeveloperNotification`（4 类**互斥**字段，仅一个出现）：
```jsonc
{
  "version": "1.0",
  "packageName": "com.voiceroom.android",
  "eventTimeMillis": "1746788688000",

  "oneTimeProductNotification": {
    "version": "1.0",
    "notificationType": 1,    // 1=ONE_TIME_PRODUCT_PURCHASED, 2=ONE_TIME_PRODUCT_CANCELED
    "purchaseToken": "oojkl...",
    "sku": "diamond_600"
  }
  // 或 voidedPurchaseNotification:
  // {
  //   "purchaseToken": "...",
  //   "orderId": "GPA.0001-...",
  //   "productType": 2,        // 1=SUBSCRIPTION, 2=ONE_TIME
  //   "refundType": 1          // 1=FULL_REFUND, 2=QUANTITY_BASED_PARTIAL_REFUND
  // }
  // 或 subscriptionNotification (Phase 2，本 Epic 不处理)
  // 或 testNotification: { "version": "1.0" }
}
```

### 9.5.2 处理规约

| 字段组合 | 服务端动作 |
|---------|----------|
| `oneTimeProductNotification.notificationType=1` (PURCHASED) | 若订单状态 IN (`PENDING`, `VERIFYING`, `PENDING_GOOGLE`)：触发 `Purchases.products:get` 走 VERIFIED → CREDITED → ACKED 路径 |
| `oneTimeProductNotification.notificationType=2` (CANCELED) | 若订单未 CREDITED → 置 FAILED；若已 CREDITED 不动（refund 走 voided） |
| `voidedPurchaseNotification` (refundType=1) | 强事务：扣回钻石 + state→REFUNDED + 写 `wallet_transactions(source='refund_google_play', amount<0)` + 风控 Sentry 告警 |
| `voidedPurchaseNotification` (refundType=2) | MVP 不支持部分退款 → 退化全额退款 + 人工跟进告警 |
| `testNotification` | 仅记录日志，HTTP 200 |
| 其他（subscription / 未知）| HTTP 200 + 仅记录日志，**不**报错（避免 Pub/Sub 重投）|

### 9.5.3 幂等与可靠性

- **去重键**：`message.messageId`，写入 `rtdn_processed` 表
- **重复消息**：直接返 200 + `outcome=ignored_duplicate`
- **失败响应**：返 5xx → Pub/Sub 自动重试（最长 7 天）；连续 24h 失败进死信队列 + Sentry P0
- **乱序**：以 `eventTimeMillis` 比对，refund 早于 purchase 到达时**先建占位记录**

---

## 9.6 错误码表（E-08）

| code | HTTP | message | 触发场景 |
|------|------|---------|---------|
| `40901` | 409 | INVALID_PURCHASE | Google 验签失败 / token 伪造 / 包名不匹配 / obfuscatedAccountId 不一致 |
| `40902` | 404 | SKU_DISABLED | SKU 不存在或已下架 |
| `40903` | 429 | ORDER_RISK_BLOCKED | 用户日失败次数 >10 / 设备黑名单 |
| `40904` | 404 | ORDER_NOT_FOUND | order_id 不存在或不属于当前用户 |
| `40905` | 409 | ORDER_ALREADY_FINALIZED | 订单已 ACKED/REFUNDED 后被尝试 verify/recredit |
| `40906` | 409 | TOKEN_REPLAY | 同一 purchase_token 已被其它订单消费 |
| `40907` | 422 | AMOUNT_MISMATCH | Google 返回金额与 SKU 配置严重不符（fraud 信号）|
| `40908` | 409 | ORDER_EXPIRED | 订单创建后 30min 未完成支付 |
| `40909` | 502 | GOOGLE_API_UNAVAILABLE | Google API 调用失败（5xx / 超时）|
| `40910` | 403 | MOCK_NOT_ALLOWED | 生产环境调用 _dev/mock_recharge |

---

## 9.7 Admin REST（Admin Server）

| 路径 | 方法 | 权限 | 说明 |
|------|------|------|------|
| `/api/v1/admin/payments/orders` | GET | `payment.read` | 列表查询（user_id / state / provider / 时间区间 / 金额区间）|
| `/api/v1/admin/payments/orders/:id` | GET | `payment.read` | 详情（含 Google 原始响应 JSON）|
| `/api/v1/admin/payments/orders/:id/recredit` | POST | `super_admin` | 补单（FAILED → CREDITED）|
| `/api/v1/admin/payments/orders/:id/refund` | POST | `super_admin` | 反向退款 |
| `/api/v1/admin/payments/skus` | GET/POST/PUT/DELETE | `payment.write` | SKU CRUD |
| `/api/v1/admin/payments/reports` | GET | `payment.report` | 财务汇总 |

字段定义详见 [admin_api.md](admin_api.md)（待 T-10025 在该文件追加章节）。

---

## 9.8 WS 信令（复用）

E-08 不引入新 WS 信令；仅复用：
- **`BalanceUpdated`** (S→C 单播) — 入账后服务端推送，详见 [websocket_signals.md §6.8.3](websocket_signals.md#683-balanceupdatedsc-单播)
  - 字段 `delta` = `+sku.diamonds`
  - 字段 `reason` = `"recharge_google_play"` \| `"dev_mock"` \| `"refund_google_play"`（Epic 内**新增**枚举值）
  - 字段 `order_id` = 关联订单 UUID（**新增**字段，可选）

---

## 9.9 BillingClient 客户端 SDK 字段对照表

> 客户端实现层强制对照（防止误用）。版本：`com.android.billingclient:billing-ktx:8.3.0`。

| Google API | 类型 | 用途 | 服务端期望 |
|-----------|------|------|----------|
| `BillingClient.newBuilder().enablePendingPurchases().enableAutoServiceReconnection().setListener(...).build()` | 构造 | 必须启用 enablePendingPurchases 与 enableAutoServiceReconnection | - |
| `queryProductDetailsAsync(QueryProductDetailsParams)` | 查询 | 仅取 `ProductType.INAPP` | - |
| `BillingFlowParams.Builder.setObfuscatedAccountId(orderId)` | 关键 | **必填** = 我方 `order_id` | 服务端 verify 时校验匹配 |
| `BillingFlowParams.Builder.setObfuscatedProfileId(userId)` | 关键 | 填我方 `user_id`（防欺诈关联）| 服务端 verify 时校验匹配 |
| `Purchase.purchaseToken` | 取值 | POST verify body | DB 主键语义 |
| `Purchase.orderId` | 取值 | POST verify body 可选字段 | 仅审计；可空（promo code）|
| `Purchase.purchaseState` | 判断 | 1=PURCHASED / 2=PENDING | 客户端不上报 PENDING（等 RTDN）|
| `Purchase.isAcknowledged()` | 判断 | 客户端兜底；服务端 ack 优先 | - |

---

## 9.10 安全红线（实现侧强制）

1. **purchaseToken 脱敏**：日志/审计中前 5 + 后 4 中间打 ****
2. **Service Account JSON**：通过 `config/{profile}.toml` 的 `Secret` 类型注入，禁止 BuildConfig
3. **obfuscatedAccountId 强校验**：`Purchases.products:get` 返回的 `obfuscatedExternalAccountId` 必须 == 我方 `order_id`
4. **金额校验**：Google 返回 `priceAmountMicros / 1_000_000 / fxRate(currency→USD) ÷ display_price_usd ∈ [0.7, 1.3]`，超出区间 → 40907 + Sentry P1
5. **3 天 acknowledge 红线**：`CREDITED` 状态超 24h 未 ACKED 触发告警；超 72h 视为已退款（按 Google 规则）

---

## 9.11 关联文档

- [phase1_payment_billing.md](../product/phase1_payment_billing.md)（产品方向）
- [模块10-Google Play 真支付 (E-08).md](../tasks/模块10-Google%20Play%20真支付%20(E-08).md)（Tasks）
- [websocket_signals.md §6.8.3](websocket_signals.md)（BalanceUpdated）
- [conventions.md §1.4](conventions.md)（错误码模块10 表）
- [data_models.md](data_models.md)（payment_skus / payment_orders / rtdn_processed 物理 schema）
