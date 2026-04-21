# Phase 1 - 埋点与观测性基建 (E-07.5)

> **版本**: v1.0
> **创建日期**: 2026-04-21
> **负责人**: PM Agent
> **对应 Epic**: E-07.5 埋点与观测性基建
> **与 E-07 关系**: **并行推进，E-07 礼物闭环上线前必须完成**
> **状态**: 🟡 设计中（方向已定，待拆 Task）

---

## 1. 战略定位：为什么必须立即做

### 1.1 营收闭环必须被数据验证
E-07 虚拟礼物闭环上线后，若没有埋点：
- ❌ 无法知道礼物面板**打开率**（按钮曝光→点击的转化）
- ❌ 无法知道礼物**选择→送出**的漏斗流失在哪一步
- ❌ 无法分析**哪些礼物滞销**，哪些价格档位超出用户钱包
- ❌ 无法追踪**大 R 用户行为路径**（最有价值人群）
- ❌ 无法评估**余额不足弹窗**是否有效引导"充值动机"

结论：**礼物不带埋点 = 营销瞎投 + 运营拍脑袋**。

### 1.2 合规与商店上架硬要求
- Google Play Console 要求上架前集成 **Crash 报告**（拒审高频原因 Top 5）
- MENA 数据主权法规（沙特 PDPL、阿联酋 DPR）要求用户行为数据**存储于境内或经授权境外**
- 审计溯源（用户投诉/客服排障）需要完整的日志链路

### 1.3 项目红线对齐
本 Epic 直接落地以下架构规约：
- ✅ [anticorruption_layer.md](../architecture/anticorruption_layer.md) - 埋点 SDK 必须走防腐层
- ✅ [observability.md](../architecture/observability.md) - 日志/指标/追踪三支柱
- ✅ [mena_localization.md](../architecture/mena_localization.md) - 数据主权要求

---

## 2. 范围边界 (Scope)

### 2.1 In Scope（本 Epic 必做）

| 领域 | 内容 | 产出 |
|------|------|------|
| **Android Crash 监控** | Sentry SDK 集成（防腐层包装）+ 自动捕获崩溃 + ANR + 非致命异常上报 | Sentry Dashboard 可见崩溃 |
| **Android 行为埋点** | 事件上报 SDK（防腐层）+ 关键事件埋点（登录/进房/送礼/充值/分享等） | 20+ 核心事件 |
| **Android 上报通道** | **复用 WS 通道批量上报**（节流+压缩+断网重试）+ 兜底 HTTP | 1 个 `EventReportClient` |
| **App Server 事件接收** | WS 信令 `ReportEvent` + HTTP `POST /api/v1/events/batch` + 异步写入 | 2 个接口 |
| **事件存储** | PostgreSQL `events` 表（MVP）+ 分区表设计（按日） | 迁移脚本 |
| **Admin 事件查询** | Admin Server 简易查询接口（按 user_id / event_name / 时间范围） | 1 个接口 |
| **Web 事件浏览页** | Admin 看板新增"用户行为"Tab，查某用户的事件流 | 1 个页面 |

### 2.2 Out of Scope（延后）

| 领域 | 延后到 | 原因 |
|------|--------|------|
| AppsFlyer / Adjust 归因 | E-08 支付上线前 | 归因主要为付费归因服务，礼物闭环阶段暂不投放广告 |
| ClickHouse / Druid 分析库 | Phase 2 | MVP 阶段 PostgreSQL 足够（日事件量 <100万） |
| 指标看板（Grafana/Prometheus） | Phase 2 | MVP 用 Admin 查询 + 手动 SQL 够用 |
| A/B 测试平台 | Phase 3 | 需要稳定用户量后再做 |
| 实时性能监控（Sentry Performance Profiling） | Phase 2 | Crash + 手动 Trace 够用 |
| 服务端 APM（Tracing） | Phase 2 | Axum 自带 `tracing` 足够 |

### 2.3 技术决策

#### 决策 1：Crash 监控选型 — **Sentry**（不用 Firebase Crashlytics）
| 维度 | Firebase Crashlytics | **Sentry（推荐）** |
|------|---------------------|-------------------|
| 数据主权 | 🔴 数据入 Google，中东合规模糊 | ✅ 支持 **自建部署**（Sentry Self-Hosted） |
| 免费额度 | 无限 | 5K events/月（初期够用） |
| Rust Server 支持 | ❌ 无 | ✅ `sentry` crate 一等公民 |
| 数据导出 | 🟡 受限 | ✅ 全量 API |
| 结论 | — | **采用** |

#### 决策 2：行为埋点 — **自建上报** + **复用 WS 通道**
**为什么不用 Firebase Analytics？**
- Firebase Analytics 数据回传 Google，中东合规模糊
- 业务事件需要与用户/房间强关联，Firebase 的 user_property 机制不够灵活
- 礼物/余额等关键指标无法离开我们自己的数仓做业务分析

**为什么复用 WS 通道？**（关键创新）
- 房间页用户 90% 时间都连着 WS，HTTP 再开连接是浪费
- 节省电量与 MENA 流量（4G 计费）
- 断网时事件自动缓存在本地，WS 重连后批量 flush
- **HTTP 兜底**：WS 不在线时（如 Splash/登录阶段）走 HTTP

```
┌─ Android EventReportClient ────────┐
│ 事件入本地队列（Room 数据库）        │
│      ↓ 节流：每 10 条或 30s        │
│   ┌──────────────────────────────┐ │
│   │  WS 在线？                    │ │
│   │  是 → WS 发 ReportEvent 批量 │ │
│   │  否 → HTTP POST /events/batch│ │
│   └──────────────────────────────┘ │
│      ↓ 成功清队列 / 失败保留      │
└────────────────────────────────────┘
```

#### 决策 3：存储 — PostgreSQL 分区表（MVP）
- 单表 `events(id, user_id, event_name, properties JSONB, client_ts, server_ts, session_id)`
- 按日分区（`events_20260421`），每日自动创建分区
- 保留 30 天（TTL 清理），历史数据归档到对象存储
- 查询走 `user_id + time_range` 联合索引

#### 决策 4：归因（AppsFlyer）— **延后到 E-08**
归因核心价值是"这次广告带来的用户最终付了多少钱"，真支付未上之前归因无数据可分析。Phase 1 MVP 阶段通过 **user_source 自报字段**（注册时记录来源）兜底。

---

## 3. 核心事件字典（MVP 20+）

详细事件规范见 [business_flows.md §2.9](./business_flows.md)。本节仅列大类：

| 事件类型 | 事件名示例 | 关键 properties |
|---------|-----------|----------------|
| 应用生命周期 | `app_launch` / `app_foreground` / `app_background` | session_id, device_info |
| 认证 | `login_request` / `login_success` / `login_fail` / `logout` | phone_region, fail_reason |
| 大厅 | `hall_view` / `room_card_click` / `create_room_click` | position, room_id |
| 房间 | `room_enter` / `room_leave` / `mic_take` / `mic_leave` | room_id, duration, slot_index |
| 聊天 | `chat_send` / `chat_receive` | msg_len, msg_type |
| **礼物（核心）** | `gift_panel_open` / `gift_select` / `gift_send_click` / `gift_send_success` / `gift_send_fail` / `gift_receive` | gift_id, count, total_price, effect_level, fail_reason |
| **钱包（核心）** | `wallet_view` / `balance_update` / `insufficient_balance` / `recharge_click` | old_balance, new_balance, source |
| 榜单 | `ranking_view` / `ranking_tab_switch` | tab_type, period |
| 房间治理（E-10 同步） | `kick_user` / `mute_user` / `ban_chat` | target_user_id, operator_role |
| 崩溃 / 异常 | （Sentry 自动） | stack_trace, device, os |

---

## 4. 关键业务流程

### 4.1 事件上报正向流程
```
1. Android UI 层触发事件（如用户点送礼成功）
   ↓
2. 调用防腐层 Analytics.track(event_name, properties)
   ↓
3. 防腐层写入本地 Room 数据库 event_queue 表
   ↓
4. Throttler 判断：队列 >=10 条 或 距上次 flush >30s ?
   ↓
5. 是 → 批量取出（最多 100 条）
   ↓
6. 判断 WS 连接状态：
   ├─ WS 在线：发送 ReportEvent { events: [...] }
   └─ WS 离线：POST /api/v1/events/batch（含 JWT）
   ↓
7. 成功 → 从本地队列删除已上报事件
   失败 → 保留队列，下次重试；队列超 1000 条则丢最旧的（防爆）
```

### 4.2 异常流程

| 场景 | 处理 |
|------|------|
| 设备存储已满 | 队列 drop 最旧事件；埋点不阻塞主流程 |
| 长期离线（飞行模式多日） | 本地保留最新 1000 条；超出丢弃 |
| Server 事件接口挂 | 客户端按指数退避重试；**不影响业务功能** |
| 用户未登录但在使用 App（Splash/登录页） | 事件用 `device_id` 关联，登录成功后服务端回填 user_id |
| 用户未授权（隐私合规） | 首启动 Splash 页弹一次权限弹窗；拒绝后仅保留 Crash 监控（合规豁免） |

---

## 5. Android 端 UI 可见性

本 Epic 主要是基建，UI 层的可见产出仅 1 个：

| UI 点 | 说明 |
|------|------|
| **首启动隐私弹窗** | Splash 页后弹出，"我们会收集你的使用数据以改进体验" + [同意] [仅 Crash] 二选一；遵循中东 PDPL 合规 |

> 其他事件埋点是**埋在现有页面里**，无独立 UI。

**Admin 端**：Web 新增"用户行为"Tab（在用户详情页），可见事件流时间线。

---

## 6. 技术约束（Plan Agent 必须遵循）

1. **防腐层强制**（红线 #3）：
   - 业务层**严禁** import `io.sentry.*` 或直接调用 Sentry API
   - 必须通过 `core/analytics/AnalyticsPort` 接口
   - Sentry 实现放在 `core/analytics/impl/SentryAnalytics.kt`

2. **性能影响**：埋点 SDK 必须 **零阻塞主线程**；写入本地 Room 走 IO dispatcher
3. **电量影响**：批量节流必须严格；单个事件严禁触发 HTTP
4. **合规默认值**：未获用户同意时，**禁止收集手机号、精确位置、通讯录**
5. **单一事实源**（红线 #1）：崩溃与关键业务事件的时间戳以 **Server 落库时间** 为准，客户端时间仅用于调试
6. **数据脱敏**：上报时**严禁**带 JWT、明文手机号、密码；`user_id` 用 UUID
7. **Sentry 自建部署**：使用 Sentry Self-Hosted（Docker Compose），或选用 Sentry SaaS EU 区域（`de.sentry.io`）

---

## 7. 验收指标 (Exit Criteria)

- [ ] Android 崩溃率 <0.5%（Sentry Dashboard 可见）
- [ ] 礼物全漏斗（面板打开→选择→送出→成功）埋点完整，日均事件量可追踪
- [ ] 事件从触发到 Admin 可查询延迟 <60s
- [ ] 网络断开 5 分钟后恢复，本地缓存事件全部成功补报
- [ ] 隐私弹窗拒绝后仅保留 Crash 事件，其他事件停止上报
- [ ] WS 通道上报占比 >80%（验证复用设计有效）
- [ ] 所有新增事件在 `doc/product/business_flows.md §2.9` 有规范定义

---

## 8. 预估任务拆解（~6 Tasks）

> 实际拆解在 PM 阶段下一步产出。本节仅预估结构。

| Task ID | 归属 | 任务名 | 预估 |
|---------|------|-------|------|
| T-00022 | App Server | 事件表 schema + 分区 + 接收 API | 6h |
| T-00023 | App Server | WS `ReportEvent` 信令 + 写入服务 | 4h |
| T-10015 | Admin Server | 用户行为查询 API | 4h |
| T-20013 | Web | 用户详情页"行为流" Tab | 5h |
| T-30034 | Android | Analytics 防腐层 + Sentry 集成 | 6h |
| T-30035 | Android | EventReportClient + 核心事件埋点 + 隐私弹窗 | 10h |

**总预估**：~35h，可与 E-07 完全并行（零依赖冲突）。

---

## 9. 文档变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v1.0 | 2026-04-21 | 初始版本：确定"复用 WS 通道上报"核心设计、Sentry 选型、MVP 事件字典 |
