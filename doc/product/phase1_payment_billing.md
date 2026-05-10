# Phase 1 - Google Play 真支付与订单中心 (E-08)

> **版本**: v1.1（基于 Google 官方文档校准）
> **创建日期**: 2026-05-09 / 修订: 2026-05-10
> **负责人**: PM Agent
> **对应 Epic**: E-08 Google Play 真支付
> **Billing Library 版本**: `com.android.billingclient:billing-ktx:8.3.0`（截至 2026-05 最新）
> **官方权威参考（必读，禁止偏离）**:
> - 接入：https://developer.android.com/google/play/billing/integrate
> - 测试：https://developer.android.com/google/play/billing/test
> - 安全（服务端验证）：https://developer.android.com/google/play/billing/security
> - RTDN 字段：https://developer.android.com/google/play/billing/rtdn-reference
> **前置 Epic**: E-07 虚拟礼物与钱包闭环（✅ Done） / E-07.5 埋点观测（✅ Done）
> **继任 Epic**: E-09 贵族体系（依赖钻石购买能力）
> **状态**: 🟡 设计中

---

## 1. 战略定位

### 1.1 为什么必须做
E-07 已经把"钻石→礼物→榜单"这条**消费侧**主动脉打通，但充值通道目前只有 **Admin 手动调整余额**，营收完全依赖运营人工灌量，**无法规模化**。

E-08 的目标是：**让用户自助完成"真金→钻石"的全自动入账闭环**，把 ARPU 曲线从 0 拉到 $4-6（参考 [competitors.md](./competitors.md) MENA 地区基线）。

### 1.2 通道选择：Google Play Billing 优先
| 通道 | 接入难度 | 合规风险 | MENA 覆盖率 | 优先级 |
|------|---------|---------|------------|--------|
| **Google Play Billing 8.x** | 中 | 低（Google 自身合规） | Android 90%+ | **P0（本 Epic）** |
| Apple IAP | 中 | 低 | iOS（暂不支持） | Phase 2（iOS 端开工后） |
| STC Pay（沙特本地钱包） | 高 | 中（牌照） | 沙特 70%+ | Phase 3（专项 Epic） |
| Mada / KNet（沙特/科威特银行卡） | 高 | 高（PCI-DSS） | 全 GCC 60%+ | Phase 3 |
| 加密货币（USDT/Web3） | 中 | 极高（监管） | 极小众 | 不做 |

> **结论**：MVP 只做 Google Play Billing。复用同一套 `payment_orders` 与 `wallet_transactions` 表，后续接入其他通道仅需新增 `provider` 枚举值与回调路由。

### 1.3 防欺诈与合规红线
1. **强 Server 验签**：客户端取到的 `purchaseToken` **必须**回 Server，调 [Purchases.products:get](https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.products/get) 获取 `purchaseState`（`0=PURCHASED, 1=CANCELED, 2=PENDING`，引自 Google API 字段）与 `acknowledgementState`，**严禁**信任客户端 `success` 回调。
2. **幂等入账**：以 **`purchaseToken` 作为唯一键**（**注意**：Google 官方安全文档 [security#token](https://developer.android.com/google/play/billing/security) 明确指出，`orderId` 在 promo code 兑换场景下可能为空，因此**严禁**用 orderId 当主键）。DB 约束：`UNIQUE(provider, purchase_token)`；`provider_order_id` 仅做审计冗余字段。
3. **强事务**：Google 订单核验通过 → 写订单 → 加余额 → 写 wallet_transactions → ack Google，必须在同一个 SQLx Transaction 里。
4. **沙箱与生产隔离**：Google Play License Key、Service Account 走 `config/{profile}.toml`，**严禁**硬编码到 BuildConfig。
5. **税与汇率**：Google 已含税到账金额（`priceAmountMicros`）作为审计基准；钻石数 = SKU 配置表的固定挂单数（不做实时汇率换算，避免风控）。

---

## 2. 范围边界 (Scope)

### 2.1 In Scope
| 领域 | 内容 |
|------|------|
| SKU 配置 | `payment_skus` 表（sku_id / provider=google_play / diamonds / display_price_usd / is_active / sort_order） |
| 订单中心 | `payment_orders` 表（order_id / user_id / sku_id / provider / provider_order_id / purchase_token / amount_micros / currency / state / created_at / acked_at） |
| 创建订单 | POST `/api/v1/payments/orders` 服务端预创建订单（防重 + 风控）→ 返回内部 order_id |
| 校验回调 | POST `/api/v1/payments/google/verify`（客户端主动） + Google RTDN（异步推送，Phase 1.5 接入） |
| 余额入账 | 强事务：状态机 `PENDING → VERIFIED → CREDITED → ACKED` |
| 失败补偿 | 客户端持久化未 acknowledge 的 purchaseToken，App 启动时全量重试 verify |
| Admin 订单查询 | 列表/筛选/详情，支持 user_id/状态/时间区间 |
| Admin 手动补单 | 仅 super_admin，写 admin_logs，复用 `T-10013 调整余额` 链路 |
| Web 财务报表 | 日/月成交额（按货币聚合 + USD 折算汇总） |
| Android 充值页 | SKU 列表、Billing 唤起、订单状态轮询、充值历史 |

### 2.2 Out of Scope（延后）
| 领域 | 延后到 | 原因 |
|------|--------|------|
| Apple IAP | iOS 立项后 | 当前无 iOS 端 |
| 沙特本地钱包 STC Pay | E-08.5 / Phase 3 | 合规周期 3+ 月 |
| 提现（主播 → 现金） | 独立 Epic | 涉及 KYC/AML/银行通道 |
| 优惠券 / 首充奖励 | E-08.5 | MVP 先打通主流程 |
| 退款流程 | Phase 2 | 退款率 < 1%，先用 Admin 手动补单兜底 |

---

### 1.4 真实购买生命周期对照表（Google → 内部）

> **底线**：客户端可见的 Google `getPurchaseState()` 仅有 `PURCHASED(1)` / `PENDING(2)` 两种值；**严禁**在客户端构造其它状态。内部 7 状态机是服务端**自定义的工作流**，与 Google 客户端 SDK 状态一一映射如下：

| 内部状态 | 触发 | 对应 Google 字段 |
|---------|------|----------------|
| PENDING（内部预下单） | Server `POST /payments/orders` 写入 | 尚未发起 Billing |
| VERIFYING | 客户端 `PurchasesUpdatedListener.onPurchasesUpdated()` 收到 `BillingResponseCode.OK` 后回传 token | `Purchase.purchaseState = 1 (PURCHASED)` 但 `acknowledgementState = 0` |
| VERIFIED | 服务端调 `Purchases.products:get` 返回 `purchaseState=0(PURCHASED)` 且 `obfuscatedExternalAccountId` 匹配 | 同上 |
| CREDITED | 服务端事务完成入账 | 同上（acknowledge 尚未调用） |
| ACKED（终态） | 服务端调 `Purchases.products:acknowledge` 或 `:consume` 成功 | `acknowledgementState = 1` |
| FAILED | 验签失败 / token 重放 / amount 不匹配 | - |
| REFUNDED | 收到 RTDN `OneTimeProductNotification.notificationType = 2 (ONE_TIME_PRODUCT_CANCELED)` 或 `VoidedPurchaseNotification` | - |

**关于 PENDING 购买**：当用户使用现金/银行转账等延迟支付方式时，客户端 `getPurchaseState()` 返回 `2 (PENDING)`。**红线**：PENDING 状态下**严禁授权钻石**（官方 [integrate#pending](https://developer.android.com/google/play/billing/integrate) 明令）；待 RTDN 推送 `ONE_TIME_PRODUCT_PURCHASED` 后再走 VERIFYING → CREDITED 流程。

---

## 3. 订单状态机

```
                ┌─ Google Play 弹窗用户取消 ──► CANCELLED (终态)
                │
   PENDING ─────┤                                              ┌─► 验签失败 ──► FAILED (终态)
   (内部预创建)  │                                              │
                └─ 客户端 PurchasesUpdatedListener
                   收到 PURCHASED(1) ──► VERIFYING ─────┤
                                                               └─► Purchases.products:get OK
                                                                   └ purchaseState=0
                                                                   └ obfuscatedExternalAccountId 匹配
                                                                   └ productId 匹配预下单
                                                                   ─► VERIFIED
                                                                       │
                                                                       ▼ 强事务：扣单+加余额+写流水
                                                                   CREDITED
                                                                       │
                                                                       ▼ 调 Purchases.products:acknowledge / :consume（3 天内）
                                                                    ACKED (终态)
```

**关键约束**：
- `CREDITED → ACKED` 之间宕机不影响用户，下次启动重试 acknowledge（**Google 官方 3 天宽限期，超过自动退款** —— [integrate#process](https://developer.android.com/google/play/billing/integrate)）。
- `VERIFIED → CREDITED` **禁止**重入，依赖 `UNIQUE(provider, purchase_token)` 兼底。
- 任何处于 `VERIFYING` 超 10 分钟的订单 → 后台 cron 拉 `Purchases.products:get` 状态做对账。
- **订阅型订单**（Phase 2 使用）需额外处理 `linkedPurchaseToken`：如果 subscription:get 响应中出现该字段，必须同时将上一个 token 从本库删除并撤销授权（[security#linked](https://developer.android.com/google/play/billing/security)）。

---

## 4. 业务流程

### 4.1 充值正向流程
```
Android 用户点击"充值" → 拉 SKU 列表（GET /payments/skus）
   → 选档位（如 6 USD = 600 钻石）→ 客户端调 Server 预下单（POST /payments/orders）
   → Server 写 PENDING 订单，返回 order_id
   → 客户端调 BillingClient.launchBillingFlow(sku_id, order_id 作为 obfuscatedAccountId)
   → Google Play 弹窗 → 用户付款成功 → onPurchasesUpdated 回调 purchaseToken
   → 客户端 POST /payments/google/verify { order_id, purchase_token }
   → Server 调 Google purchases.products.get → 校验 state=PURCHASED + obfuscatedAccountId 匹配
   → 强事务：状态 → CREDITED + users.diamond_balance += sku.diamonds + wallet_tx
   → Server 调 Google acknowledgePurchase → 状态 ACKED
   → WS 推送 BalanceUpdated → Android 钱包页/礼物面板余额刷新
```

### 4.2 关键异常流

| 场景 | Server 行为 | Android UI |
|------|-----------|-----------|
| 用户取消支付 | 订单标 CANCELLED | 充值页恢复，无提示 |
| Google 验签失败（伪造 token） | 返回 40901 INVALID_PURCHASE，订单 FAILED | 弹窗"支付校验失败，请联系客服" + 写 Sentry |
| obfuscatedAccountId 不匹配（用户 A 票据被用户 B 上传） | 订单 FAILED + 风控告警 | 弹窗"订单异常" + 拒绝入账 |
| 重复 purchaseToken | 幂等返回首次结果 + 客户端 consume 该 token | 跳到"充值成功"页 |
| 客户端 acknowledge 前杀进程 | 订单停在 CREDITED；下次启动客户端发现未 acknowledge 的 token，主动重试 verify | 用户已收到余额，无感知 |
| Google 订单 3 天未 acknowledge | Google 自动退款（Server 监听 RTDN 退款事件，扣回余额） | Phase 1.5 RTDN 接入后实现 |
| 网络断开 | 客户端持久化 purchaseToken 到 DataStore，恢复后重试 verify | "正在校验充值"loading + 5 分钟后超时引导联系客服 |
| 余额变更广播丢失 | Android 充值页轮询订单状态，状态=ACKED 时强拉 wallet/balance | 充值成功后必然刷新 |

### 4.3 风控规则（MVP 内置）
| 规则 | 阈值 | 处置 |
|------|------|-----|
| 单用户日下单失败次数 | > 10 次 | 24h 拒绝创建新订单（Redis 计数） |
| 单用户日成交金额 | > $1000 | 风控告警（Sentry + Web 高亮），不阻塞 |
| 同 purchaseToken 验签次数 | > 5 次 | 拒绝并写黑名单 |
| 设备 ID + IP 一日新建账户充值 | > 3 个 | 触发人工审核 flag（不阻塞，标记字段） |

---

## 5. 关键技术约束

1. **强事务**（红线 #2）：订单状态推进 + 余额加 + 流水写入 = 同一 SQLx Transaction
2. **幂等键**（红线 #2）：`UNIQUE(provider, provider_order_id)` + 客户端请求带 `Idempotency-Key`（复用 T-00044 模式）
3. **防腐层**（红线 #3）：`BillingPort` 接口隔离 Google Play SDK，便于 Mock 与未来扩展 Apple IAP
4. **配置隔离**（红线 #4）：Service Account JSON、License Key 走 `config/{profile}.toml` + `Secret` 类型脱敏日志
5. **观测性**：每个状态推进点埋 Analytics 事件（`payment_create / payment_verify / payment_credited / payment_failed`），失败原因写 Sentry tag

---

## 6. 验收指标 (Exit Criteria)

| 类别 | 指标 | 目标 |
|------|------|------|
| 功能 | Google Play 沙箱 5 档 SKU 全部跑通 | 100% |
| 性能 | 验签 P95 延迟 | < 1.5s |
| 一致性 | 订单状态机 7 天内不一致样本（CREDITED 未 ACKED 超 24h） | < 0.5% |
| 安全 | 伪造 purchaseToken 100% 拦截 | 100% |
| 用户 | 客户端宕机恢复后未 ACK 订单自动补偿成功率 | > 99% |
| 业务 | 上线 30 天 ARPPU（付费用户均值） | ≥ $20 |

---

## 7. 与其他 Epic 的接口

- **E-07**: 直接复用 `users.diamond_balance` + `wallet_transactions` + `BalanceUpdated` WS 信令
- **E-09 贵族**: 贵族购买首选支付 = 钻石；高阶贵族（公爵以上）允许人民币直购，复用本 Epic 的"创建订单"接口（多 SKU 类型 `noble_pack`）
- **E-07.5 埋点**: 全链路事件透传 `order_id`，Web 行为流可关联订单
- **Admin Server**: 复用 `admin_logs`，订单查询权限走 `T-10012` RBAC

---

## 8. 风险登记

| 风险 | 影响 | 缓解 |
|------|-----|------|
| Google Play 在沙特/伊朗等地区可用性低 | 营收受限 | E-08.5 接入 STC Pay 兜底 |
| 退款率 > 5% 触发 Google 风控 | 整店下架 | 严格风控 + 客服 SOP |
| Service Account JSON 泄露 | 资金被盗用 | Vault/KMS 托管，定期轮换 |
| 多端时序竞态（Web 改余额时用户正在支付） | 余额不一致 | DB 行级锁 `SELECT FOR UPDATE` |

---

## 9. 测试体系（基于 Google 官方）

> ⚠️ 用户 PRD 提出"如 Google Play 无测试入口则补 mock"。**官方实情**：Google Play **已提供完整测试体系**（链接见 §0 文档头部 [test 文档](https://developer.android.com/google/play/billing/test)）。下表逐项落地：

### 9.1 Google 官方测试机制（生产/Staging 主用）

| 机制 | 用途 | 接入步骤 | 覆盖 Task |
|------|------|---------|----------|
| **License Testers** | 真机内部跑 IAP 全链路（不真扣款）| Play Console → 设置 → 许可测试人员 → 添加测试账号；上传任意 track 的签名 APK；测试机用同一 Google 账号登录 | T-30060/61/62 验证流程 |
| **Test card, always approves** | 模拟"即时购买成功" | License Tester 登录后 launchBillingFlow 自动出现"测试卡"选项 | T-30062 verify 成功路径 |
| **Test card, always declines** | 模拟"购买被拒"（onPurchasesUpdated 回 BillingResponseCode.ITEM_UNAVAILABLE 等）| 同上，选另一张卡 | T-30062 失败路径 |
| **Slow test cards (delayed approve / delayed decline)** | 模拟 PENDING 状态，几分钟后自动转 PURCHASED 或 CANCELED | 同上选 slow 卡 | T-30063 PENDING UI + RTDN 处理路径 |
| **Play Billing Lab Android App**（包名 `com.google.android.apps.play.billingtestcompanion`）| 切换 Play 国家、加速订阅周期、试用价格变更 | 测试机安装并以 license tester 账号登录；配置 2h 有效 | Phase 2 订阅测试预备 |
| **Internal Test Track** | 团队 ≤100 人灰度，绕开 Play 审核 | Console → 测试 → 内部测试 → 拉测试群组 | 全 E-08 集成测试 |
| **加速订阅续费测试表** | 1 周→5min；1 月→5min；3 月→10min；6 月→15min；1 年→30min | License tester 账号自动加速 | Phase 2 订阅 |
| **3 分钟自动退款** | License tester 未 acknowledge 的购买 3 分钟后自动退款，便于反复跑 RTDN refund 路径 | 仅许可测试人员账号生效 | T-00054 RTDN refund 处理 |

### 9.2 Dev/CI 环境 Mock 通道（**与 9.1 互补，不冲突**）

**为什么仍需补 Mock**：
- 9.1 全部依赖 **Play 服务可用 + 已上传 Console 的签名 APK**；本地 Docker、CI 流水线、模拟器无 Play Service 时**完全不可用**
- 单元测试 / 集成测试需要确定性（slow card 的"几分钟"对 CI 不友好）
- Reviewer 排障时需要可重放的"成功/失败"切换开关

**实现规范**（详见 T-00055 / T-30065，本 Epic 新增 Tasks）：
| 维度 | 约束 |
|------|------|
| **作用域** | 仅 `dev` + `staging` 配置；**生产编译产物零代码**（productFlavor 源码集 + Cargo `#[cfg(feature = "dev_payment_mock")]` 双层隔离） |
| **服务端入口** | POST `/api/v1/_dev/mock_recharge { user_id, sku_id, force_outcome: "success" \| "fail" \| "pending" }`；config `payment.mock_enabled=true` 才注册路由；生产构建 feature flag 关闭，访问返回 404 |
| **客户端入口** | Android `dev`/`staging` flavor 的"开发者菜单 → 测试购买"，跳过 BillingClient，直接调上面 API；`testTag = "debug_mock_recharge_${outcome}"` |
| **链路复用** | Mock 走与生产相同的订单/钻石/流水代码路径（`BillingPort` 防腐层注入 `MockBillingPort` 替代 `GooglePlayBillingPort`），保证测试行为与真实链路 1:1 |
| **审计** | Mock 充值在 `wallet_transactions.source` 字段标记为 `dev_mock`，Admin 端报表自动剔除该来源，避免污染财务数据 |
| **风险阻断** | 启动时若 `profile=production` 而 `mock_enabled=true`，**直接 panic**（防配置错放）|

### 9.3 测试金字塔分配

| 层级 | 工具 | 覆盖场景 |
|------|------|---------|
| 单测（Server）| `cargo test` + `MockBillingPort` | 状态机 7 状态、幂等键、风控规则 |
| 单测（Android）| JUnit + Robolectric + 假 BillingClient | ViewModel 状态切换、retry 逻辑 |
| 集成测试（CI）| `dev_payment_mock` feature + 9.2 Mock 通道 | 端到端订单流、RTDN 重放 |
| 真机回归（Staging）| 9.1 License Testers + always approves/declines/slow | UI 交互、Play 服务真实信号 |
| 灰度（Pre-prod）| 9.1 Internal Test Track + 真支付小额 SKU | 真实 RTDN、真实退款 |
| 生产（Live）| Sentry + Analytics + 财务报表 | 监控异常率、退款率、ack 滞后 |

---

## 10. RTDN（Real-Time Developer Notifications）完整契约

> 官方权威：[rtdn-reference](https://developer.android.com/google/play/billing/rtdn-reference)

### 10.1 接入

1. Cloud Console 创建 Pub/Sub Topic `voiceroom-rtdn-{env}`
2. Play Console → 货币化设置 → 实时开发者通知 → 填入 topic 全名 + 点"发送测试通知"
3. 我方 Server 订阅 Push 端点 `POST /webhook/rtdn`（HTTPS + Service Account OIDC token 验签）

### 10.2 消息结构（**字段名禁止改造**，以官方 schema 为准）

```jsonc
{
  "message": {
    "messageId": "136969346945",   // Pub/Sub 全局唯一，**用作幂等去重键**
    "publishTime": "2026-05-09T10:24:48.690Z",
    "data": "<base64 of DeveloperNotification JSON>"
  },
  "subscription": "projects/voiceroom/subscriptions/rtdn-sub"
}
```

base64 解出后的 `DeveloperNotification`（官方四种通知**互斥**，仅含其中之一）：

```jsonc
{
  "version": "1.0",
  "packageName": "com.voiceroom.android",
  "eventTimeMillis": "1746788688000",
  // —— 一次性商品（钻石包） ——
  "oneTimeProductNotification": {
    "version": "1.0",
    "notificationType": 1,        // 1=ONE_TIME_PRODUCT_PURCHASED, 2=ONE_TIME_PRODUCT_CANCELED
    "purchaseToken": "oojkl...",
    "sku": "diamond_600"
  }
  // —— 退款 —— （与上互斥，单独一条消息）
  // "voidedPurchaseNotification": {
  //   "purchaseToken": "...",
  //   "orderId": "GPA.0001-...",
  //   "productType": 2,            // 1=SUBSCRIPTION, 2=ONE_TIME
  //   "refundType": 1              // 1=FULL_REFUND, 2=QUANTITY_BASED_PARTIAL_REFUND
  // }
  // —— 测试通知 ——（点"发送测试通知"或灰度时）
  // "testNotification": { "version": "1.0" }
}
```

### 10.3 处理规则

| notificationType | 内部动作 |
|-----------------|---------|
| `oneTimeProductNotification.notificationType=1` (PURCHASED) | 若订单仍 PENDING/VERIFYING：触发 `Purchases.products:get` → VERIFIED → CREDITED → ACKED |
| `oneTimeProductNotification.notificationType=2` (CANCELED) | 若订单未 CREDITED：直接置 FAILED；若已 CREDITED 走 voidedPurchaseNotification 通道（理论不会单独发 type=2 给已发货订单）|
| `voidedPurchaseNotification`（refundType=1 全额）| 强事务：扣回钻石（允许负余额，记账时打 `negative_balance_reason=refund`）→ 状态 REFUNDED → 风控告警 |
| `voidedPurchaseNotification`（refundType=2 部分）| 当前 MVP **不支持部分退款**（Google 主要用于多数量购买）→ 退化为全额扣回 + Sentry 告警人工跟进 |
| `testNotification` | 仅记日志，HTTP 200 |

### 10.4 幂等与可靠性

- **去重键**：`messageId`，写入 `rtdn_processed(message_id PRIMARY KEY, processed_at)` 表，重复消息直接 200
- **失败重试**：处理失败返 4xx/5xx，Pub/Sub 自动重试（最长 7 天）；连续失败进死信队列
- **顺序**：Pub/Sub **不保证顺序**，业务层以 `eventTimeMillis` 比对 + 状态机吸收乱序（refund 在 purchase 前到达 → 先建空 PURCHASED 占位再扣）

---

## 11. 关联文档

- [Tasks 模块 10 - E-08 Google Play 真支付](../tasks/模块10-Google%20Play%20真支付%20(E-08).md)
- [架构文档 - 防腐层规约](../architecture/anticorruption_layer.md)
- [协议文档 - 错误码](../protocol/error_codes.md)（新增 40901 INVALID_PURCHASE / 40902 SKU_DISABLED / 40903 ORDER_RISK_BLOCKED）
- [设计稿 - Android 充值页 T-30060](../design/android/T-30060.md)
- [设计稿 - Android 充值历史 T-30064](../design/android/T-30064.md)
- Google 官方权威（必读）：[integrate](https://developer.android.com/google/play/billing/integrate) · [test](https://developer.android.com/google/play/billing/test) · [security](https://developer.android.com/google/play/billing/security) · [rtdn-reference](https://developer.android.com/google/play/billing/rtdn-reference)
