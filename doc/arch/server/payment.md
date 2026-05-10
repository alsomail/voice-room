# Server 端：支付模块（E-08 Google Play 真支付）

**Last Updated:** 2026-05-10 · **Status:** DoD ✅

## 模块职责

处理 Google Play In-App Billing 的完整支付闭环：创建订单 → Google 验签 → 钻石入账 → RTDN 退款处理。

## 目录结构

```
app/server/src/modules/payment/
├── controller.rs       # HTTP handlers（create_order, verify, rtdn_webhook）
├── service.rs          # PaymentOrderService（创建订单 + 风控）
├── verify_service.rs   # PaymentVerifyService（验签 + 强事务入账）
├── rtdn_service.rs     # PaymentRtdnService（RTDN + RS256 OIDC 验签）
├── repo.rs             # DB 操作（SELECT FOR UPDATE 并发安全）
├── risk.rs             # RiskCheckService（24h 失败 > 10 限流）
├── google_billing_port.rs  # GooglePlayBillingPort trait（防腐层）
├── cron.rs             # 对账 cron（PENDING 24h/PENDING_GOOGLE 72h 终态推进）
├── routes.rs           # 路由注册
└── dev_mock/           # dev_payment_mock feature（仅 dev/staging）
    ├── controller.rs
    └── service.rs

app/server/migrations/
└── 012_create_payment_tables.sql  # payment_skus / payment_orders / rtdn_processed
```

## 状态机

```
PENDING ──[verify ok]──→ VERIFYING ──[ack ok]──→ CREDITED ──[ack done]──→ ACKED
                             ↓
                        [purchaseState=2]
                             ↓
                        PENDING_GOOGLE ──[cron 72h]──→ ACKED/FAILED

PENDING ──[cron 24h timeout]──→ CANCELLED
VERIFYING ──[verify fail]──→ FAILED
PENDING_GOOGLE ──[cron 72h]──→ FAILED
CREDITED ──[RTDN refund]──→ REFUNDED
```

**关键约束**：
- 状态不可逆向
- UPDATE 均带 `AND state=? WHERE order_id=?` 约束防乐观锁失败
- PENDING→VERIFYING→CREDITED 强事务三阶段不可中断（断电恢复重复调用则推进）

## 关键设计决策

### 1. 防腐层
`GooglePlayBillingPort` trait 隔离所有 Google SDK 依赖：
- 生产实现：调用 Google Purchases API + RS256 OIDC 验签（RTDN webhook）
- Dev/Staging Mock 实现：`dev_payment_mock` feature flag，仅 feature 启用时链接

### 2. 并发安全
- `SELECT FOR UPDATE` 两阶段锁防双充
- 订单创建：`INSERT IGNORE` 或 UNIQUE 约束 `(provider, provider_order_id)`
- verify + credit：再次查询订单状态校验，防止中间状态被他人修改
- 强事务链路：DB transaction 包含查询→状态检查→更新钻石→写流水→推送四步，任意失败则全部回滚

### 3. RTDN 安全
- RS256 + Google JWKS 端点验签（缓存 1h）
- issuer/audience 双重校验：
  - issuer = "https://accounts.google.com"
  - audience = "com.yourcompany.yourapp"（来自 Firebase Console）
- message_id 幂等：`rtdn_processed(message_id) PRIMARY KEY` 防重复入账

### 4. 幂等性
- 创建订单：`UNIQUE(provider, provider_order_id)` 约束
- verify 幂等：相同 purchase_token 重复调用返回首次结果
- RTDN 幂等：message_id 作 PK，消息重复到达则 UPDATE IGNORE

### 5. Dev Mock
- `dev_payment_mock` Cargo feature + `payment.mock_enabled` 配置双开关
- production profile 启动时若 `mock_enabled=true` 则 panic（红线）
- Mock 订单标记 `provider='mock'`，写 wallet_transactions 时 `source='dev_mock'` 隔离财务报表

## 🔌 协议入口索引

| # | 协议类型 | 客户端入口 | URL / 信令 | 服务端处理函数 | protocol/ 锚点 |
|---|---------|---------|-----------|--------------|---------------|
| 1 | HTTP POST | Android `RechargeRepository.kt::createOrder` ⭐ | `POST /api/v1/payments/orders` | `payment::controller::create_order_handler` | [payment_api.md §9.3.1](../protocol/payment_api.md#931-创建订单-post-apiv1paymentsorders) |
| 2 | HTTP POST | Android `RechargeRepository.kt::verifyPurchase` ⭐ | `POST /api/v1/payments/google/verify` | `payment::controller::verify_handler` | [payment_api.md §9.3.2](../protocol/payment_api.md#932-验签与入账-post-apiv1paymentsgoogleverify) |
| 3 | HTTP POST | Google Cloud Pub/Sub Push（无客户端，服务端接收）| `POST /api/v1/payments/google/rtdn` | `payment::controller::rtdn_webhook_handler` | [payment_api.md §9.4](../protocol/payment_api.md#94-rtdn-推送-post-apiv1paymentsgooglertdn) |
| 4 | WS S→C 单播 | — | `BalanceUpdated` reason=`recharge_google_play` | wallet::service 内部触发 | [websocket_signals.md §6.13.1](../protocol/websocket_signals.md#6131-balanceupdatedsc-单播) |
| 5 | WS S→C 单播 | — | `BalanceUpdated` reason=`refund_google_play` | RTDN CANCEL 触发 | [websocket_signals.md §6.13.1](../protocol/websocket_signals.md#6131-balanceupdatedsc-单播) |
| 6 | HTTP POST (dev) | Android dev flavor `DevToolsRepository.kt::mockRecharge` ⭐ | `POST /api/v1/_dev/mock_recharge` | `payment::dev_mock::controller::mock_recharge_handler` | [payment_api.md §9.5](../protocol/payment_api.md#95-devmock-充值通道-post-apiv1_devmock_recharge) |

## 错误码

| 代码 | HTTP | 含义 | 触发场景 |
|------|------|------|---------|
| 40901 | 422 | INVALID_PURCHASE | purchaseToken 伪造、obfuscatedAccountId 不匹配、purchaseState ≠ PURCHASED |
| 40902 | 404 | SKU_NOT_FOUND | SKU 不存在或已下架 |
| 40903 | 409 | ORDER_RISK_BLOCKED | 风控：24h 内失败订单 > 10，或设备黑名单 |
| 40904 | 409 | ORDER_ALREADY_PROCESSED | 订单已处理（CREDITED/ACKED/FAILED），禁止重复 verify |
| 40905 | 422 | INSUFFICIENT_BALANCE | 仅用于 RTDN 扣款时余额不足（此时应回滚订单） |
| 40906 | 422 | RTDN_SIGNATURE_INVALID | RTDN Webhook 签名验证失败 |
| 40907 | 422 | AMOUNT_MISMATCH | 金额 vs USD 汇率合理性检验失败 |

详见 [payment_api.md §9.10](../protocol/payment_api.md#910-安全红线实现侧强制)

## 延后项（非阻塞）

- **GooglePlayBillingPort 生产实现完整对接**（当前已有防腐层，生产实现接入 Google Purchases API 待续）
- **RTDN 生产 JWKS 缓存更新策略**（当前 1h 缓存，可提升为动态更新）
- **Sentry 告警接入**（伪造 token、金额异常等 P1 事件的实时告警）

## 关联文档

- 产品方向：[phase1_payment_billing.md](../product/phase1_payment_billing.md)
- 协议规范：[payment_api.md](../protocol/payment_api.md)
- 任务子表：[模块10-Google Play 真支付 (E-08).md](../tasks/模块10-Google%20Play%20真支付%20(E-08).md)
