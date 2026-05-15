# 模块 10: Google Play 真支付 (E-08)

> 返回 [任务总索引](./index.md)
> **产品方向**: [phase1_payment_billing.md](../product/phase1_payment_billing.md)
> **依赖**: 模块 6 (E-07 钱包闭环) ✅ Done

## Phase 1: 营收闭环 — 真支付通道

> 把 E-07 的"运营手动充值"升级为"用户自助 Google Play 充值"，打通真营收。

```
T-00050 (订单/SKU schema) ──┬─► T-00051 (创建订单 API) ──► T-30062 (Android 唤起 Billing)
                            ├─► T-00052 (Google 验签 + 入账事务) ──► T-30063 (客户端 verify+ack)
                            └─► T-10025 (Admin 订单查询) ──► T-20030 (Web 订单列表)
T-00052 ──► T-00053 (RTDN 异步对账, P1.5)
T-00050 ──► T-10026 (Admin 手动补单) ──► T-20031 (Web 补单弹窗)
T-00050 ──► T-10027 (Admin SKU CRUD) ──► T-20032 (Web SKU 管理页)
T-30060 (充值页 UI) ──► T-30061 (Billing 防腐层) ──► T-30062 ──► T-30063 ──► T-30064 (历史)
```

---

## App Server

| Task ID | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate | QA Gate | Overall Gate |
|---------|------|----------|----------|----------|-------------|----------|------------|----------|-------------|---------|--------------|
| **T-00050** | Payment | 订单与 SKU Schema 与迁移 | T-00017 (wallet) | 新建 `payment_skus` (sku_id PK / provider / diamonds / display_price_usd / is_active / sort_order) 与 `payment_orders` (order_id UUID PK / user_id / sku_id / provider / provider_order_id / purchase_token / amount_micros / currency / state ENUM / created_at / verified_at / credited_at / acked_at)；`UNIQUE(provider, provider_order_id)`；插入 5 档 SKU 种子（60/300/600/1980/6480 钻石） | 1. 迁移可幂等执行<br>2. UNIQUE 约束阻止重复入账<br>3. 状态枚举 PENDING/VERIFYING/VERIFIED/CREDITED/ACKED/CANCELLED/FAILED<br>4. 索引 (user_id, created_at DESC) | 4h | Done | Done | ✅ | - | - |
| **T-00051** | Payment | 创建订单 API + 风控 | T-00050 | POST `/api/v1/payments/orders { sku_id }`，写 PENDING 订单，返回 internal `order_id`（作 obfuscatedAccountId）；执行风控（日失败 > 10 / 设备 ID 黑名单）；返回 SKU 详情 | 1. 同一用户并发创建 5 个订单全部成功<br>2. 风控触发返回 40903<br>3. SKU 不存在返回 40902<br>4. 响应含 order_id + sku 完整字段 | 5h | Done | Done | ✅ | - | - |
| **T-00052** | Payment | Google Play 验签 + 入账强事务 [P0] | T-00051 | POST `/api/v1/payments/google/verify { order_id, purchase_token }`：调 Google `purchases.products.get`（防腐层 `GooglePlayBillingClient`）→ 校验 `purchaseState=PURCHASED` + `obfuscatedAccountId == order_id`；强事务推进 PENDING→VERIFIED→CREDITED + `users.diamond_balance += sku.diamonds` + 写 wallet_transactions(type='recharge')；调 acknowledgePurchase → ACKED；广播 BalanceUpdated | 1. 沙箱 5 档全跑通<br>2. 重复 purchase_token 幂等返回<br>3. 伪造 token 返回 40901 + 写 Sentry<br>4. obfuscatedAccountId 不匹配返回 40901<br>5. 强事务断电后状态自洽（CREDITED 未 ACK 重启可继续 ack） | 12h | Done | Done | ✅ | - | - |
| **T-00053** | Payment | RTDN 推送对账 + 退款处理 | T-00052 | POST `/api/v1/payments/google/rtdn`（Pub/Sub 推送端点）：接收 Google Real-Time Developer Notifications；解析 ONE_TIME_PRODUCT 通知；状态 SUBSCRIPTION_CANCELLED/REFUNDED 时扣回余额（事务）；签名验证（X-Goog-* 头） | 1. 验证消息签名<br>2. 退款扣余额事务原子性<br>3. 重复消息幂等<br>4. 不识别消息类型告警不报错 | 8h | Done | Done | ✅ | - | - |
| **T-00054** | Payment | 待 ACK 订单后台对账 cron | T-00052 | Tokio cron 每 5min 扫 `state=VERIFYING > 10min` 或 `state=CREDITED AND acked_at IS NULL > 1h` 订单，重新查 Google 状态强制推进 | 1. 不影响主流程性能<br>2. 失败有 Sentry 告警<br>3. 推进结果写 admin_logs | 4h | Done | Done | ✅ | - | - |
| **T-00055** | Payment | Dev/Staging Mock 充值通道 [Dev-only] | T-00050, T-00052 | 新增 POST `/api/v1/_dev/mock_recharge { user_id, sku_id, force_outcome: "success"\|"fail"\|"pending" }`；Cargo `feature = "dev_payment_mock"` + config `payment.mock_enabled` 双开关才注册路由；服务启动时若 `profile=production` 且 `mock_enabled=true` 直接 panic；入账走与 T-00052 同一事务路径（注入 `MockBillingPort`）；`wallet_transactions.source = 'dev_mock'` 隔离财务报表；依据 [phase1_payment_billing.md §9.2](../product/phase1_payment_billing.md) | 1. production 构建产物不含该路由（feature flag 隔离）<br>2. profile=production + mock_enabled=true 启动不能起服<br>3. force_outcome=success 入账 + 广播 BalanceUpdated<br>4. force_outcome=pending 后调 二次 force_outcome=success 完成<br>5. force_outcome=fail 订单状态 FAILED + 不入账<br>6. 同一 (user_id, sku_id) 1秒内重复调用安静幂等<br>7. 访问未认证返 401 | 4h | Done | Done | ✅ | - | - |

---

## Admin Server

| Task ID | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate | QA Gate | Overall Gate |
|---------|------|----------|----------|----------|-------------|----------|------------|----------|-------------|---------|--------------|
| **T-10025** | Order | 订单查询 API | T-00050, T-10012 | GET `/api/v1/admin/payments/orders`：分页 + 过滤 (user_id / state / provider / created_at 区间 / 金额区间)；GET `/orders/:id` 详情含完整状态时间戳与 Google 原始响应 JSON；写 audit_log 操作 | 1. 分页 P95 < 200ms<br>2. 跨租户隔离<br>3. 详情含状态机历史<br>4. 无权限 403 | 5h | TDD | **Done** | ✅ Passed | - | - |
| **T-10026** | Order | 手动补单 / 退款 API | T-10025, T-10013 | POST `/api/v1/admin/payments/orders/:id/recredit { reason }` 仅 super_admin：将 FAILED 订单置 CREDITED + 加余额（复用 T-10013 调整余额链路）；POST `/refund { reason }` 反向操作；强双签确认 | 1. 仅 super_admin 通过<br>2. reason 必填<br>3. 写 admin_logs (operator_id + reason)<br>4. 已 ACKED 订单禁止再补单返回 40904 | 5h | TDD | **Done** | ✅ Passed | - | - |
| **T-10027** | SKU | SKU CRUD API | T-00050, T-10012 | `/api/v1/admin/payments/skus` GET/POST/PUT；上下架开关 is_active；价格/钻石数变更必须二次确认；DELETE 软删；写 admin_logs | 1. 价格 > 0、钻石 > 0 校验<br>2. sku_id 与 Google Console 一致性提醒（暂仅 warning）<br>3. 软删后用户已下单不影响入账<br>4. 操作落 audit | 4h | TDD | **Done** | ✅ Passed | - | - |
| **T-10028** | Report | 财务汇总 API | T-00050 | GET `/api/v1/admin/payments/reports?granularity=day|month&from=&to=`：聚合成交额（按货币分组 + USD 折算）+ 订单数 + 退款数 + 平均客单价 | 1. 100w 订单聚合 < 2s<br>2. 时区按 Asia/Riyadh<br>3. 退款金额带负号<br>4. 折算汇率配置化 | 5h | TDD | **Done** | ✅ Passed | - | - |

---

## Web Admin

| Task ID | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate | QA Gate | Overall Gate | UI 设计文档 |
|---------|------|----------|----------|----------|-------------|----------|------------|----------|-------------|---------|--------------|------------|
| **T-20030** | Order | 订单列表与详情页 | T-10025, T-20007 | 新建路由 `/admin/payments/orders`：表格 + 多条件筛选（用户/状态/时间）+ CSV 导出；行点击侧边 Drawer 显示状态机时间线 + Google 原始响应 JSON | 1. 状态彩色标签<br>2. 失败原因显著高亮<br>3. 时间线含 7 个状态节点<br>4. 列表 a11y 通过 | 8h | TDD | **Done** | ✅ Passed | - | - | 待补 |
| **T-20031** | Order | 补单/退款弹窗 | T-20030, T-10026 | 订单详情 Drawer 增加"补单""退款"按钮 (super_admin only)：弹窗强双确认（输入金额 + 原因）；操作后实时刷新订单 | 1. 非 super_admin 按钮 hidden<br>2. 二次确认输入"CONFIRM"才允许提交<br>3. 失败 Toast 显示 errorCode<br>4. 成功后状态/时间线即时刷新 | 4h | TDD | **Done** | ✅ Passed | - | - | 待补 |
| **T-20032** | SKU | SKU 管理页 | T-10027, T-20007 | 新建路由 `/admin/payments/skus`：表格 CRUD + 上下架开关 + 价格变更二次确认 | 1. 软删行灰显示<br>2. 钻石/价格输入校验<br>3. sku_id 与 Google 不一致 warning 黄条<br>4. 操作记录可追溯 | 5h | TDD | **Done** | ✅ Passed | - | - | 待补 |
| **T-20033** | Report | 财务报表页 | T-10028, T-20007 | 新建路由 `/admin/payments/reports`：日/月切换 + 折线图（成交额/订单数/退款率） + Top SKU 表格；CSV 导出 | 1. 100w 订单不卡<br>2. 切换粒度数据正确<br>3. 折线 Tooltip 显示完整明细<br>4. RTL 模式排版正常 | 6h | TDD | **Done** | ✅ Passed | - | - | 待补 |

---

## Android

| Task ID | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate | QA Gate | Overall Gate | UI 设计文档 |
|---------|------|----------|----------|----------|-------------|----------|------------|----------|-------------|---------|--------------|------------|
| **T-30060** | Wallet | 充值页 UI（SKU 列表） | T-30027 (钱包页) | 在钱包页"充值"按钮替换占位 Toast，点击进入 `RechargeScreen`：顶部当前余额；SKU 卡片网格（2 列）含钻石数 + 价格 + "热门"角标；底部"充值历史"入口 | 1. 默认拉 GET `/payments/skus`<br>2. SKU 加载 Skeleton<br>3. 选中卡片金色边框<br>4. 网络失败重试按钮<br>5. RTL 兼容 | 6h | TDD | **Done** | ✅ Passed | - | - | [T-30060.md](../design/android/T-30060.md) |
| **T-30061** | Payment | Billing 防腐层 (BillingPort) | T-30060 | 新增 `IBillingPort` 接口 + `GooglePlayBillingAdapter`（封装 BillingClient v6+ 的 connect/queryProductDetails/launchBillingFlow/onPurchasesUpdated/consume/acknowledge）；`FakeBillingPort` 用于单测 | 1. 接口仅暴露领域语义<br>2. 业务层无 com.android.billingclient 直接 import（红线 #3）<br>3. Adapter 单测覆盖错误码映射<br>4. CI 静态检查脚本拦截违规 import | 8h | TDD | **Done** | ✅ Passed | - | - | 无 |
| **T-30062** | Payment | 创建订单 + 唤起 Billing | T-30060, T-30061, T-00051 | 点击 SKU 卡片 →（loading）调 POST `/payments/orders` 拿 order_id → BillingPort.launch(sku_id, obfuscatedAccountId=order_id) → 监听 onPurchasesUpdated 取 purchaseToken | 1. order_id 与 obfuscatedAccountId 严格一致<br>2. 用户取消支付订单标记 CANCELLED 不弹错<br>3. 重复点击防抖 1s<br>4. 风控 40903 友好提示 | 6h | TDD | **Done** | ✅ Passed | - | - | [T-30060.md](../design/android/T-30060.md) |
| **T-30063** | Payment | 客户端 verify + ack + 容错 | T-30062, T-00052 | onPurchasesUpdated 后：① 写 DataStore `pending_purchases` ② 调 verify ③ 收到 ACKED → BillingPort.acknowledge → 删 DataStore 记录；App 启动时 `PendingPurchaseResumer` 全量重试未完成的 token | 1. 杀进程后冷启动可恢复<br>2. 验证失败 5min 重试 3 次<br>3. 同 token 永不重复消费<br>4. 充值成功 Toast + 跳钱包页<br>5. 余额由 BalanceUpdated 推送刷新（红线 #1） | 8h | TDD | **Done** | ✅ Passed | - | - | [T-30060.md](../design/android/T-30060.md) |
| **T-30064** | Wallet | 充值历史页 | T-30060, T-00018 | 新建 `RechargeHistoryScreen`：复用 wallet/transactions API 过滤 type=recharge；显示订单 ID/钻石数/金额/状态/时间；"待入账"项点击重试 verify | 1. 分页加载<br>2. 状态彩色标签<br>3. PENDING 项可重试<br>4. 与 Web 端 T-20030 数据一致 | 4h | TDD | **Done** | ✅ Passed | - | - | [T-30064.md](../design/android/T-30064.md) |
| **T-30065** | Payment | Dev/Staging 开发者菜单：测试购买入口 [Dev-only] | T-00055 | Android `dev` 与 `staging` flavor 独立 sourceSet 下提供隐藏入口（个人中心连击版本号 7 下售卖），打开 `DevToolsScreen` 含"模拟充值-成功"/"-失败"/"-PENDING"三个按钮，直接调 T-00055 mock API，跳过 BillingClient；production flavor 源码集不包含该页面（类不存在）；UI 顶部黄条"仅限开发环境 / 不影响财务报表" | 1. production APK 反编译无 `DevToolsScreen` 类<br>2. `success` 点击后钱包余额实际增加<br>3. `fail` 点击后 Toast 失败原因<br>4. `pending` 点击后状态呈现入 "处理中" + 金额未增加<br>5. testTag `debug_mock_recharge_${outcome}` 可被自动化定位 | 3h | TDD | **Done** | ✅ Passed | - | - | 无 |

---

## 模块汇总
- **App Server**: 6 Tasks（42h）
- **Admin Server**: 4 Tasks（19h）
- **Web Admin**: 4 Tasks（23h）
- **Android**: 6 Tasks（35h）
- **总计**: 20 Tasks ≈ 119h（其中 T-00055 / T-30065 为 Dev-only 不影响生产）

## 实现路径建议
1. **Sprint 1**：T-00050 → T-00051 → T-30060 → T-30061（基础设施）
2. **Sprint 2**：T-00052 → T-30062 → T-30063（核心闭环，沙箱跑通）
3. **Sprint 2.5**：T-00055 + T-30065（Dev/CI Mock 通道，为后续 Sprint 提供離线可重复测试能力）
4. **Sprint 3**：T-10025/26/27 → T-20030/31/32（管理端）
5. **Sprint 4**：T-00054 + T-30064 + T-10028 + T-20033（对账与报表）
6. **Sprint 5（可选）**：T-00053 RTDN 接入（异步退款监控）
